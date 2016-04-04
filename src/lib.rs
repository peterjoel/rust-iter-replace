
use std::collections::{BTreeSet, BTreeMap, VecDeque};
use std::cell::RefCell;

///
pub struct Replace <'a, I, T: 'a + Ord > {
    iter: I,
    buffer_out: VecDeque<T>,
    buffer_in: Vec<T>,
    replace_states: Vec<ReplaceState<'a, T>>,
    index: usize,
    flushed_index: usize,
}

pub struct Replacement <'a, T: 'a + Ord> {
    search_for: &'a [T],
    replace_with: &'a [T],
}

impl <'a, T: 'a + Ord> Replacement <'a, T> {
    pub fn new(search_for: &'a [T], replace_with: &'a [T]) -> Replacement<'a, T> {
        Replacement {
            search_for: search_for,
            replace_with: replace_with,
        }
    }
}

struct ReplaceState <'a, T: 'a + Ord> {
    search_for: &'a [T],
    replace_with: &'a [T],
    candidates: RefCell<BTreeSet<usize>>,
}

impl <'a, T: 'a + Ord> ReplaceState <'a, T> {
    fn new(search_for: &'a [T], replace_with: &'a [T]) -> ReplaceState<'a, T> {
        ReplaceState {
            search_for: search_for,
            replace_with: replace_with,
            candidates: RefCell::new(BTreeSet::new()),
        }
    }
}


impl <'a, I, T> Replace <'a, I, T> where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {

    fn adapt(iter: I, replace_states: Vec<ReplaceState<'a, T>>) -> Replace<'a, I, T> {
        Replace {
            iter: iter,
            buffer_out: VecDeque::new(),
            buffer_in: Vec::new(),
            replace_states: replace_states,
            index: 0,
            flushed_index: 0,
        }
    }

    fn fill_buffer(&mut self) {
        'consume: while let Some(item) = self.iter.next() {

            self.index += 1;

            // buffer all incoming items
            self.buffer_in.push(item);

            for replace_state in self.replace_states.iter() {

                let mut candidates = replace_state.candidates.borrow_mut();

                // Prune existing partial match candidates that don't match the current item
                let removes: Vec<_> = candidates.iter().cloned()
                    .filter(|start_index| {
                        replace_state.search_for[self.index - *start_index] != item
                    }).collect();
                for r in removes {
                    candidates.remove(&r);
                }

                // Keep track of new partial match candidates
                if replace_state.search_for[0] == item {
                    candidates.insert(self.index);
                }
            }

            let index = self.index;
            let flush_index = self.calc_flushable_index();

            let matching_term = self.replace_states.iter().find(|replace_state| {
                let mut candidates = replace_state.candidates.borrow_mut();
                candidates.iter().cloned()
                    .next()
                    .into_iter()
                    .find(|x| index - x + 1 == replace_state.search_for.len())
                    .is_some()
            });

            match matching_term {
                None => {
                    if flush_index > self.flushed_index {
                        let unflushed = flush_index - self.flushed_index;
                        let mut flush: VecDeque<_> = self.buffer_in.drain(0 .. unflushed).collect();
                        self.buffer_out.append(&mut flush);
                        self.flushed_index = flush_index;
                        break 'consume;
                    }
                },
                Some(replace_state) => {
                    // A match! So replace it and clear all the partial matches
                    for replace_state in self.replace_states.iter() {
                        let mut candidates = replace_state.candidates.borrow_mut();
                        candidates.clear();
                    }
                    for &x in replace_state.replace_with.iter() {
                        self.buffer_out.push_back(x);
                    }
                    self.buffer_in.clear();
                    self.flushed_index = self.index;
                    break 'consume;
                }
            }
        }
    }

    // the smallest index into buffer_in that doesn't contain a match
    fn calc_flushable_index(&mut self) -> usize {
        self.replace_states.iter().map(|replace_state| {
            let mut candidates = replace_state.candidates.borrow_mut();
            candidates.iter()
                .next()
                .map(|x| x - 1)
                .unwrap_or(self.index)
            }).min().unwrap_or(0)
    }

}


pub trait ReplaceIter<'a, I, T> where
    I: Iterator<Item = T>,
    T: Ord {

    fn replace(self, search_for: &'a [T], replace_with: &'a [T]) -> Replace<'a, I, T>;

    fn replace_all(self, replacements: Vec<Replacement<'a, T>>) -> Replace<'a, I, T>;

}

impl <'a, I, T> ReplaceIter<'a, I, T> for I where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {

    ///
    fn replace(self, search_for: &'a [T], replace_with: &'a [T]) -> Replace<'a, I, T> {
        let mut states = Vec::with_capacity(1);
        states.push(ReplaceState::new(search_for, replace_with));
        Replace::adapt(self, states)
    }

    fn replace_all(self, replacements: Vec<Replacement<'a, T>>) -> Replace<'a, I, T> {
        let states = replacements.iter()
            .map(|state| ReplaceState::new(state.search_for, state.replace_with))
            .collect();
        Replace::adapt(self, states)
    }
}

impl <'a, I, T> Iterator for Replace <'a, I, T> where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {

    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.buffer_out.len() == 0 {
            self.fill_buffer();
        }
        self.buffer_out.pop_front()
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_replace_simple() {
        let v: Vec<u32> = vec![1,2,3].into_iter().replace(&[2], &[10]).collect();
        assert_eq!(v, vec![1,10,3]);
    }

    #[test]
    pub fn test_replace_longer() {
        let v: Vec<u32> = vec![3,4,5,6,7,8,9].into_iter().replace(&[4,5], &[100]).collect();
        assert_eq!(v, vec![3,100,6,7,8,9]);
    }

    #[test]
    pub fn test_replace_multi_matches() {
        let v: Vec<u32> = vec![3,4,5,6,4,5,9].into_iter().replace(&[4,5], &[100,200,300]).collect();
        assert_eq!(v, vec![3,100,200,300,6,100,200,300,9]);
    }

    #[test]
    pub fn test_nearly_match() {
        let v: Vec<u32> = vec![3,4,5,6].into_iter().replace(&[4,5,1], &[100,200]).collect();
        assert_eq!(v, vec![3,4,5,6]);
    }

    #[test]
    pub fn test_replace_overlapping() {
        let v: Vec<u32> = vec![3,4,5,4,5,4,9].into_iter().replace(&[4,5,4,5], &[100]).collect();
        assert_eq!(v, vec![3,100,4,9]);
    }

    #[test]
    pub fn test_replace_all_single(){
        let reps = vec![Replacement::new(b"ab", b"AB")];
        let v: Vec<u8> = b"abcacab".iter().cloned().replace_all(reps).collect();
        assert_eq!(v.as_slice(), b"ABcacAB");
    }

    #[test]
    pub fn test_many_replacements(){
        let reps = vec![Replacement::new(b"abc", b"_ABC_"),
                        Replacement::new(b"de", b"_DE_")];
        let v: Vec<u8> = b"ababcdef".iter().cloned().replace_all(reps).collect();
        assert_eq!(v.as_slice(), b"ab_ABC__DE_f");
    }

    #[test]
    pub fn test_overlapping_patterns_in_declared_order(){
        let reps = vec![Replacement::new(b"ab", b"_AB_"),
                        // ignored because the previous one will always match first
                        Replacement::new(b"abc", b"_ABC_")];
        let v: Vec<u8> = b"abcabc".iter().cloned().replace_all(reps).collect();
        assert_eq!(v.as_slice(), b"_AB_c_AB_c");
    }
}
