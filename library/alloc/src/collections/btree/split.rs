use core::alloc::Allocator;
use core::borrow::Borrow;

use super::node::ForceResult::*;
use super::node::{Handle, Root};
use super::search::SearchResult::*;

/// Maximum height whose split path we record on the stack.
///
/// With `B = 6`, a height of 32 already exceeds the number of elements that
/// can exist in a 64-bit address space.
const MAX_SPLIT_HEIGHT: usize = 32;

impl<K, V> Root<K, V> {
    /// Calculates the length of both trees that result from splitting up
    /// a given number of distinct key-value pairs.
    pub(super) fn calc_split_length(
        total_num: usize,
        root_a: &Root<K, V>,
        root_b: &Root<K, V>,
    ) -> (usize, usize) {
        let (length_a, length_b);
        if root_a.height() < root_b.height() {
            length_a = root_a.reborrow().calc_length();
            length_b = total_num - length_a;
            debug_assert_eq!(length_b, root_b.reborrow().calc_length());
        } else {
            length_b = root_b.reborrow().calc_length();
            length_a = total_num - length_b;
            debug_assert_eq!(length_a, root_a.reborrow().calc_length());
        }
        (length_a, length_b)
    }

    /// Split off a tree with key-value pairs at and after the given key.
    /// The result is meaningful only if the tree is ordered by key,
    /// and if the ordering of `Q` corresponds to that of `K`.
    /// If `self` respects all `BTreeMap` tree invariants, then both
    /// `self` and the returned tree will respect those invariants.
    ///
    /// `Ord`/`Borrow` are invoked only while locating the split path, before
    /// any key-value pair is moved. A panic in the comparator therefore leaves
    /// `self` unchanged.
    pub(super) fn split_off<Q: ?Sized + Ord, A: Allocator + Clone>(
        &mut self,
        key: &Q,
        alloc: A,
    ) -> Self
    where
        K: Borrow<Q>,
    {
        let left_root = self;
        let height = left_root.height();

        // Pass 1: record the split edge at every level. `search_node` is the
        // only call that invokes `Ord`/`Borrow` and can panic. The tree is
        // not mutated yet, so a panic here leaves `self` intact.
        let mut split_edges = [0usize; MAX_SPLIT_HEIGHT];
        let mut depth = 0;
        {
            let mut node = left_root.reborrow();
            loop {
                assert!(
                    depth < MAX_SPLIT_HEIGHT,
                    "BTreeMap height exceeds split_off stack buffer"
                );
                let idx = match node.search_node(key) {
                    // key is going to the right tree
                    Found(kv) => kv.idx(),
                    GoDown(edge) => edge.idx(),
                };
                split_edges[depth] = idx;
                depth += 1;
                match node.force() {
                    Internal(internal) => {
                        // SAFETY: `idx` came from `search_node` on this node.
                        node = unsafe { Handle::new_edge(internal, idx) }.descend();
                    }
                    Leaf(_) => break,
                }
            }
        }
        debug_assert_eq!(depth, height + 1);

        // Pass 2: replay the recorded edges. This does not invoke `Ord`.
        let mut right_root = Root::new_pillar(height, alloc.clone());
        let mut left_node = left_root.borrow_mut();
        let mut right_node = right_root.borrow_mut();

        for level in 0..depth {
            // SAFETY: `split_edges[level]` was produced by `search_node` on
            // this level of the unmodified tree, so it is a valid edge index.
            let mut split_edge = unsafe { Handle::new_edge(left_node, split_edges[level]) };
            split_edge.move_suffix(&mut right_node);

            if level + 1 == depth {
                break;
            }
            match (split_edge.force(), right_node.force()) {
                (Internal(edge), Internal(node)) => {
                    left_node = edge.descend();
                    right_node = node.first_edge().descend();
                }
                _ => unreachable!(),
            }
        }

        left_root.fix_right_border(alloc.clone());
        right_root.fix_left_border(alloc);
        right_root
    }

    /// Creates a tree consisting of empty nodes.
    fn new_pillar<A: Allocator + Clone>(height: usize, alloc: A) -> Self {
        let mut root = Root::new(alloc.clone());
        for _ in 0..height {
            root.push_internal_level(alloc.clone());
        }
        root
    }
}
