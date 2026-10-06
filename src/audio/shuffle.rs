use rand::{RngExt, SeedableRng, rngs::StdRng, seq::SliceRandom};

#[derive(Default, Debug)]
pub struct ShuffleState {
    seed: u64,
    index: usize,
    pending_seed: Option<u64>,
}

impl ShuffleState {
    pub fn reset(&mut self) {
        self.seed = rand::rng().random::<u64>();
        self.index = 0;
        self.pending_seed = None;
    }

    pub fn next(&mut self, queue_len: usize, repeat: bool, peek: bool) -> Option<i32> {
        if queue_len == 0 {
            return None;
        }

        if self.index < queue_len {
            let next = Self::order(self.seed, queue_len)[self.index];
            if !peek {
                self.index += 1;
            }
            return Some(next as i32);
        }

        if !repeat {
            if !peek {
                self.reset();
            }
            return None;
        }

        let seed = *self
            .pending_seed
            .get_or_insert_with(|| rand::rng().random::<u64>());
        let next = Self::order(seed, queue_len)[0];
        if !peek {
            self.seed = self.pending_seed.take().unwrap();
            self.index = 1;
        }
        Some(next as i32)
    }

    pub fn previous(
        &mut self,
        queue_len: usize,
        current_index: i32,
        position: u64,
        repeat: bool,
    ) -> Option<i32> {
        if queue_len == 0 {
            return None;
        }

        if position > 3 {
            // restart current song
            return Some(current_index);
        }

        let order = Self::order(self.seed, queue_len);
        if self.index > 1 {
            let previous = *order.get(self.index - 2)?;
            self.index -= 1;
            Some(previous as i32)
        } else if repeat {
            self.index = queue_len;
            Some(order[queue_len - 1] as i32)
        } else {
            self.index = 1;
            Some(order[0] as i32)
        }
    }

    fn order(seed: u64, queue_len: usize) -> Vec<usize> {
        let mut order: Vec<_> = (0..queue_len).collect();
        order.shuffle(&mut StdRng::seed_from_u64(seed));
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(seed: u64, index: usize, pending_seed: Option<u64>) -> ShuffleState {
        ShuffleState {
            seed,
            index,
            pending_seed,
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Snapshot {
        seed: u64,
        index: usize,
        pending_seed: Option<u64>,
    }

    fn snapshot(state: &ShuffleState) -> Snapshot {
        Snapshot {
            seed: state.seed,
            index: state.index,
            pending_seed: state.pending_seed,
        }
    }

    #[test]
    fn default_starts_at_zero_without_pending_cycle() {
        assert_eq!(
            snapshot(&ShuffleState::default()),
            Snapshot {
                seed: 0,
                index: 0,
                pending_seed: None,
            }
        );
    }

    #[test]
    fn each_repeat_cycle_plays_every_song_once() {
        let queue_len = 9;
        let mut shuffle = state(11, 0, None);

        for seed in [11, 22, 33, 44] {
            if shuffle.index == queue_len {
                shuffle.pending_seed = Some(seed);
            }
            let expected = ShuffleState::order(seed, queue_len);
            let actual: Vec<_> = (0..queue_len)
                .map(|_| shuffle.next(queue_len, true, false).unwrap() as usize)
                .collect();
            assert_eq!(actual, expected); // order is correct
            let mut sorted = actual;
            // check that each idex only appears once: 1, 2, ..., queue_len - 1
            sorted.sort_unstable();
            assert_eq!(sorted, (0..queue_len).collect::<Vec<_>>());
        }
    }

    #[test]
    fn peek_does_not_consume() {
        let mut shuffle = state(42, 2, None);
        let before = snapshot(&shuffle);
        let expected = Some(ShuffleState::order(42, 7)[2] as i32);

        for _ in 0..3 {
            assert_eq!(shuffle.next(7, true, true), expected);
            assert_eq!(snapshot(&shuffle), before);
        }
    }

    #[test]
    fn next_steps_forward() {
        let mut shuffle = state(42, 2, None);
        let expected = Some(ShuffleState::order(42, 7)[2] as i32);

        assert_eq!(shuffle.next(7, true, false), expected);
        assert_eq!(
            snapshot(&shuffle),
            Snapshot {
                seed: 42,
                index: 3,
                pending_seed: None,
            }
        );
    }

    #[test]
    fn boundary_peek_keeps_current_cycle() {
        let mut shuffle = state(42, 7, None);
        let prefetched = shuffle.next(7, true, true); // peeking at last index
        let pending = shuffle.pending_seed.expect("peek must reserve a cycle");
        // peeked value is coming from the next seed cycle
        assert_eq!(prefetched, Some(ShuffleState::order(pending, 7)[0] as i32));
        assert_eq!(
            snapshot(&shuffle),
            Snapshot {
                seed: 42,
                index: 7,
                pending_seed: Some(pending),
            }
        );

        for _ in 0..3 {
            assert_eq!(shuffle.next(7, true, true), prefetched);
            // snapshot remains the same after peek
            assert_eq!(
                snapshot(&shuffle),
                Snapshot {
                    seed: 42,
                    index: 7,
                    pending_seed: Some(pending),
                }
            );
        }
        assert_eq!(shuffle.next(7, true, false), prefetched);
        // Not peeked, wrap back to the first index
        assert_eq!(
            snapshot(&shuffle),
            Snapshot {
                seed: pending,
                index: 1,
                pending_seed: None,
            }
        );
    }

    #[test]
    fn previous_steps_back() {
        let mut shuffle = state(42, 4, Some(91));
        let order = ShuffleState::order(42, 7);

        assert_eq!(
            shuffle.previous(7, order[3] as i32, 3, true),
            Some(order[2] as i32)
        );
        assert_eq!(
            snapshot(&shuffle),
            Snapshot {
                seed: 42,
                index: 3,
                pending_seed: Some(91),
            }
        );
    }

    #[test]
    fn previous_wraps_back_without_resetting() {
        let order = ShuffleState::order(42, 7);
        for index in [0, 1] {
            let mut shuffle = state(42, index, Some(91));
            assert_eq!(
                shuffle.previous(7, order[0] as i32, 0, true),
                Some(order[6] as i32)
            );
            assert_eq!(
                snapshot(&shuffle),
                Snapshot {
                    seed: 42,
                    index: 7,
                    pending_seed: Some(91),
                }
            );
            assert_eq!(
                shuffle.previous(7, order[6] as i32, 0, true),
                Some(order[5] as i32)
            );
            assert_eq!(
                snapshot(&shuffle),
                Snapshot {
                    seed: 42,
                    index: 6,
                    pending_seed: Some(91),
                }
            );
        }
    }

    #[test]
    fn previous_without_repeat_restarts() {
        let first_song = ShuffleState::order(42, 7)[0] as i32;
        for index in [0, 1] {
            let mut shuffle = state(42, index, Some(91));
            assert_eq!(shuffle.previous(7, first_song, 0, false), Some(first_song));
            assert_eq!(
                snapshot(&shuffle),
                Snapshot {
                    seed: 42,
                    index: 1,
                    pending_seed: Some(91),
                }
            );
        }
    }

    #[test]
    fn late_previous_restarts_current_song() {
        for repeat in [false, true] {
            for current_index in 0..7 {
                let mut shuffle = state(42, 0, Some(91));
                assert_eq!(
                    shuffle.previous(7, current_index, 4, repeat),
                    Some(current_index)
                );
                assert_eq!(
                    snapshot(&shuffle),
                    Snapshot {
                        seed: 42,
                        index: 0,
                        pending_seed: Some(91),
                    }
                );
            }
        }
    }

    #[test]
    fn test_empty_queue() {
        let mut shuffle = state(42, 0, None);
        assert_eq!(shuffle.next(0, true, false), None);
        assert_eq!(shuffle.previous(0, 0, 0, true), None);
    }

    #[test]
    fn one_song_can_repeat_and_wrap_backward() {
        let mut shuffle = state(42, 0, None);
        for _ in 0..4 {
            assert_eq!(shuffle.next(1, true, true), Some(0));
            assert_eq!(shuffle.next(1, true, false), Some(0));
            assert_eq!(shuffle.index, 1);
            assert_eq!(shuffle.pending_seed, None);
            let before = snapshot(&shuffle);
            assert_eq!(shuffle.previous(1, 0, 0, true), Some(0));
            assert_eq!(snapshot(&shuffle), before);
        }
    }

    #[test]
    fn reset_clears_cursor_and_pending_cycle() {
        let mut shuffle = state(42, 7, Some(91));
        shuffle.reset();
        assert_eq!(shuffle.index, 0);
        assert_eq!(shuffle.pending_seed, None);
        let expected = Some(ShuffleState::order(shuffle.seed, 7)[0] as i32);
        assert_eq!(shuffle.next(7, false, false), expected);
        assert_eq!(shuffle.index, 1);
    }

    #[test]
    fn next_without_repeat_stops_at_end() {
        let mut shuffle = state(42, 7, None);
        let before = snapshot(&shuffle);

        assert_eq!(shuffle.next(7, false, true), None);
        assert_eq!(snapshot(&shuffle), before);

        assert_eq!(shuffle.next(7, false, false), None);
        assert_eq!(shuffle.index, 0);
    }
}
