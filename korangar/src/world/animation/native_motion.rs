//! Higher actor-state motion programs recovered from the reference Ragexe.
//!
//! These programs sit above ACT playback. They select an action group and a
//! specific motion for a number of actor-update calls, independently of the
//! selected ACT action's authored delay. Their wall-clock deadline is tracked
//! separately and leaves the final selected motion held when it expires.

use ragnarok_packets::JobId;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct NativeMotionStep {
    pub(super) action_base_offset: usize,
    pub(super) motion_index: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum NativeMotionRepeat {
    Hold,
    Cycle,
}

#[derive(Clone, Debug)]
pub(super) struct NativeMotionProgram {
    steps: Vec<NativeMotionStep>,
    cadence_updates: u32,
    duration_ms: u32,
    repeat: NativeMotionRepeat,
    update_counter: u32,
    current_index: usize,
    step_serial: u32,
    complete: bool,
}

impl NativeMotionProgram {
    fn new(steps: Vec<NativeMotionStep>, cadence_updates: u32, duration_ms: u32, repeat: NativeMotionRepeat) -> Self {
        debug_assert!(!steps.is_empty());
        debug_assert!(cadence_updates > 0);

        Self {
            steps,
            cadence_updates,
            duration_ms,
            repeat,
            // Ragexe's program helper initializes the counter to one.
            update_counter: 1,
            current_index: 0,
            step_serial: 0,
            complete: false,
        }
    }

    fn held(action_base_offset: usize, motion_index: usize, cadence_updates: u32, duration_ms: u32) -> Self {
        Self::new(
            vec![NativeMotionStep {
                action_base_offset,
                motion_index,
            }],
            cadence_updates,
            duration_ms,
            NativeMotionRepeat::Hold,
        )
    }

    fn sequence(action_base_offset: usize, motion_indices: &[usize], cadence_updates: u32, duration_ms: u32) -> Self {
        Self::new(
            motion_indices
                .iter()
                .copied()
                .map(|motion_index| NativeMotionStep {
                    action_base_offset,
                    motion_index,
                })
                .collect(),
            cadence_updates,
            duration_ms,
            NativeMotionRepeat::Hold,
        )
    }

    fn range(
        action_base_offset: usize,
        start_motion: usize,
        maximum_motion: usize,
        terminal_action_base_offset: usize,
        terminal_motion: usize,
        cadence_updates: u32,
        duration_ms: u32,
    ) -> Self {
        let mut steps: Vec<_> = (start_motion..=maximum_motion)
            .map(|motion_index| NativeMotionStep {
                action_base_offset,
                motion_index,
            })
            .collect();
        // The native range helper has a distinct terminal pair. Preserve it
        // even when it duplicates the maximum range motion.
        steps.push(NativeMotionStep {
            action_base_offset: terminal_action_base_offset,
            motion_index: terminal_motion,
        });
        Self::new(steps, cadence_updates, duration_ms, NativeMotionRepeat::Hold)
    }

    fn cycle(action_base_offset: usize, motion_indices: &[usize], cadence_updates: u32, duration_ms: u32) -> Self {
        Self::new(
            motion_indices
                .iter()
                .copied()
                .map(|motion_index| NativeMotionStep {
                    action_base_offset,
                    motion_index,
                })
                .collect(),
            cadence_updates,
            duration_ms,
            NativeMotionRepeat::Cycle,
        )
    }

    pub(super) fn current_step(&self) -> NativeMotionStep {
        self.steps[self.current_index]
    }

    pub(super) fn step_serial(&self) -> u32 {
        self.step_serial
    }

    pub(super) fn is_complete(&self) -> bool {
        self.complete
    }

    /// Advance once per actor update. Deliberately do not catch up from wall
    /// time: the native cadence counter counts update calls, while only the
    /// program deadline uses elapsed milliseconds.
    pub(super) fn update(&mut self, elapsed_ms: u32) {
        if self.complete {
            return;
        }

        if elapsed_ms >= self.duration_ms {
            self.complete = true;
            return;
        }

        if self.update_counter >= self.cadence_updates {
            let next_index = match self.repeat {
                NativeMotionRepeat::Hold => (self.current_index + 1).min(self.steps.len() - 1),
                NativeMotionRepeat::Cycle => (self.current_index + 1) % self.steps.len(),
            };
            if next_index != self.current_index {
                self.current_index = next_index;
                self.step_serial = self.step_serial.wrapping_add(1);
            }
            self.update_counter = 1;
        } else {
            self.update_counter += 1;
        }
    }
}

/// Motion program installed by the shared actor-state dispatcher
/// (`0x008BAB30`). `random_variant` is the already-sampled native two-way
/// branch used by states 37, 38, and 41.
pub(super) fn shared_state_program(
    actor_state: i8,
    action_base_offset: usize,
    job_id: JobId,
    random_variant: bool,
) -> Option<NativeMotionProgram> {
    let program = match actor_state {
        // Native state 16 uses terminal action zero as its ping-pong marker.
        16 => NativeMotionProgram::cycle(action_base_offset, &[1, 2, 3, 2], 10, 99_999_999),
        17 => NativeMotionProgram::cycle(action_base_offset, &[1, 2, 3], 10, 99_999_999),
        18 => NativeMotionProgram::held(action_base_offset, 0, 10, 3_000),
        19 => NativeMotionProgram::sequence(action_base_offset, &[0, 0, 0, 1, 1], 10, 3_000),
        20 => NativeMotionProgram::new(
            vec![
                NativeMotionStep {
                    action_base_offset,
                    motion_index: 1,
                },
                NativeMotionStep {
                    action_base_offset,
                    motion_index: 1,
                },
                NativeMotionStep {
                    action_base_offset,
                    motion_index: 1,
                },
                NativeMotionStep {
                    action_base_offset,
                    motion_index: 1,
                },
                NativeMotionStep {
                    action_base_offset: 1,
                    motion_index: 5,
                },
            ],
            10,
            3_000,
            NativeMotionRepeat::Hold,
        ),
        21 => NativeMotionProgram::held(action_base_offset, 0, 10, 2_000),
        22 => NativeMotionProgram::held(action_base_offset, 2, 10, 2_000),
        23 => NativeMotionProgram::held(action_base_offset, 3, 10, 2_000),
        24 => NativeMotionProgram::held(action_base_offset, 4, 10, 2_000),
        25 => NativeMotionProgram::held(action_base_offset, 1, 10, 2_000),
        26 => {
            let motions: &[usize] = match job_id.0 {
                4049 | 4227 | 4240 | 4242 => &[4, 5, 6, 7, 2],
                _ => &[2, 3, 4, 4, 4],
            };
            NativeMotionProgram::sequence(action_base_offset, motions, 4, 500)
        }
        27 => NativeMotionProgram::held(action_base_offset, 5, 10, 3_000),
        28 => NativeMotionProgram::sequence(action_base_offset, &[0, 1, 2, 3, 4], 10, 1_000),
        30 => NativeMotionProgram::held(action_base_offset, 1, 10, 2_000),
        // The helper deliberately switches away from state 31's initial
        // shared group and drives raw action 0x50 (flat Attack2 group 10).
        31 => NativeMotionProgram::sequence(10, &[3, 5, 7, 8, 2], 5, 500),
        32 => NativeMotionProgram::sequence(action_base_offset, &[4, 5, 6, 7, 2], 5, 500),
        33 => NativeMotionProgram::held(action_base_offset, 2, 10, 1_000),
        34 => NativeMotionProgram::held(action_base_offset, 3, 10, 1_000),
        35 => NativeMotionProgram::range(action_base_offset, 0, 1, action_base_offset, 1, 6, 500),
        36 => NativeMotionProgram::range(action_base_offset, 2, 5, action_base_offset, 5, 6, 1_000),
        37 => NativeMotionProgram::held(action_base_offset, usize::from(random_variant) + 2, 10, 500),
        38 if random_variant => NativeMotionProgram::sequence(action_base_offset, &[3, 2, 1, 0, 0], 6, 1_000),
        38 => NativeMotionProgram::range(action_base_offset, 0, 3, action_base_offset, 3, 6, 1_000),
        39 | 48 => NativeMotionProgram::range(action_base_offset, 4, 5, action_base_offset, 5, 6, 500),
        40 => NativeMotionProgram::range(action_base_offset, 0, 2, action_base_offset, 2, 6, 500),
        41 => NativeMotionProgram::held(action_base_offset, usize::from(random_variant) + 1, 10, 500),
        42 => NativeMotionProgram::held(action_base_offset, 0, 10, 500),
        43 => NativeMotionProgram::held(action_base_offset, 5, 10, 500),
        44 => NativeMotionProgram::held(action_base_offset, 4, 10, 500),
        46 => NativeMotionProgram::sequence(action_base_offset, &[0, 0, 0, 1, 1], 20, 3_000),
        47 => NativeMotionProgram::cycle(action_base_offset, &[0, 1, 2, 3], 6, 9_999_999),
        49 => NativeMotionProgram::range(action_base_offset, 0, 7, action_base_offset, 7, 6, 1_000),
        50 => NativeMotionProgram::held(action_base_offset, 0, 10, 9_999_999),
        51 => NativeMotionProgram::sequence(action_base_offset, &[2, 3, 4, 4, 4], 4, 500),
        _ => return None,
    };

    Some(program)
}

/// Additional classic `CPc` state-7 programs (`0x009779A0`). Most jobs use
/// only their selected ACT action and therefore return no higher program.
pub(super) fn classic_player_state7_program(job_id: JobId) -> Option<NativeMotionProgram> {
    match job_id.0 {
        15 | 4015 | 4016 | 4020 | 4021 | 4038 | 4066 | 4070 | 4073 | 4077 => Some(NativeMotionProgram::held(12, 0, 4, 400)),
        25 | 4222 => Some(NativeMotionProgram::range(12, 2, 5, 12, 5, 4, 1_000)),
        24 | 4228 => Some(NativeMotionProgram::held(12, 3, 4, 1_000)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::JobId;

    use super::{classic_player_state7_program, shared_state_program};

    #[test]
    fn cadence_counts_actor_updates_instead_of_wall_time() {
        let mut program = shared_state_program(19, 12, JobId(7), false).unwrap();
        for elapsed in 1..10 {
            program.update(elapsed);
            assert_eq!(program.current_step().motion_index, 0);
            assert_eq!(program.step_serial(), 0);
        }
        program.update(10);
        assert_eq!(program.current_step().motion_index, 0);
        assert_eq!(program.step_serial(), 1, "the duplicate second step is still a new occurrence");
    }

    #[test]
    fn a_terminal_held_motion_does_not_create_new_event_occurrences() {
        let mut program = shared_state_program(19, 12, JobId(7), false).unwrap();
        for update in 1..=50 {
            program.update(update);
        }
        assert_eq!(program.step_serial(), 4);
        for update in 51..=100 {
            program.update(update);
        }
        assert_eq!(program.step_serial(), 4);
    }

    #[test]
    fn wall_deadline_completes_without_catching_up_steps() {
        let mut program = shared_state_program(28, 5, JobId(7), false).unwrap();
        program.update(999);
        assert!(!program.is_complete());
        assert_eq!(program.current_step().motion_index, 0);
        program.update(1_000);
        assert!(program.is_complete());
        assert_eq!(program.current_step().motion_index, 0);
    }

    #[test]
    fn ping_pong_and_cycle_programs_repeat_exact_native_orders() {
        let mut ping_pong = shared_state_program(16, 12, JobId(7), false).unwrap();
        let mut seen = vec![ping_pong.current_step().motion_index];
        for update in 1..=40 {
            ping_pong.update(update);
            if update % 10 == 0 {
                seen.push(ping_pong.current_step().motion_index);
            }
        }
        assert_eq!(seen, vec![1, 2, 3, 2, 1]);
    }

    #[test]
    fn state_twenty_switches_to_walk_group_on_its_terminal_step() {
        let mut program = shared_state_program(20, 3, JobId(7), false).unwrap();
        for update in 1..=40 {
            program.update(update);
        }
        assert_eq!(program.current_step().action_base_offset, 1);
        assert_eq!(program.current_step().motion_index, 5);
    }

    #[test]
    fn state_26_and_classic_state_7_preserve_job_specific_programs() {
        let mut linker = shared_state_program(26, 11, JobId(4049), false).unwrap();
        for update in 1..=4 {
            linker.update(update);
        }
        assert_eq!(linker.current_step().motion_index, 5);

        let gunslinger = classic_player_state7_program(JobId(24)).unwrap();
        assert_eq!(gunslinger.current_step().action_base_offset, 12);
        assert_eq!(gunslinger.current_step().motion_index, 3);
    }

    #[test]
    fn random_program_branches_are_deterministic_after_sampling() {
        assert_eq!(
            shared_state_program(37, 12, JobId(7), false).unwrap().current_step().motion_index,
            2
        );
        assert_eq!(
            shared_state_program(37, 12, JobId(7), true).unwrap().current_step().motion_index,
            3
        );
        assert_eq!(
            shared_state_program(41, 12, JobId(7), false).unwrap().current_step().motion_index,
            1
        );
        assert_eq!(
            shared_state_program(41, 12, JobId(7), true).unwrap().current_step().motion_index,
            2
        );
    }

    #[test]
    fn shared_program_domain_cadences_and_deadlines_match_dispatcher() {
        let expected = [
            (16, 10, 99_999_999),
            (17, 10, 99_999_999),
            (18, 10, 3_000),
            (19, 10, 3_000),
            (20, 10, 3_000),
            (21, 10, 2_000),
            (22, 10, 2_000),
            (23, 10, 2_000),
            (24, 10, 2_000),
            (25, 10, 2_000),
            (26, 4, 500),
            (27, 10, 3_000),
            (28, 10, 1_000),
            (30, 10, 2_000),
            (31, 5, 500),
            (32, 5, 500),
            (33, 10, 1_000),
            (34, 10, 1_000),
            (35, 6, 500),
            (36, 6, 1_000),
            (37, 10, 500),
            (38, 6, 1_000),
            (39, 6, 500),
            (40, 6, 500),
            (41, 10, 500),
            (42, 10, 500),
            (43, 10, 500),
            (44, 10, 500),
            (46, 20, 3_000),
            (47, 6, 9_999_999),
            (48, 6, 500),
            (49, 6, 1_000),
            (50, 10, 9_999_999),
            (51, 4, 500),
        ];

        let actual_states: Vec<_> = (0..=51)
            .filter(|actor_state| shared_state_program(*actor_state, 12, JobId(7), false).is_some())
            .collect();
        assert_eq!(actual_states, expected.iter().map(|(state, ..)| *state).collect::<Vec<_>>());

        for (actor_state, cadence_updates, duration_ms) in expected {
            let program = shared_state_program(actor_state, 12, JobId(7), false).unwrap();
            assert_eq!(program.cadence_updates, cadence_updates, "state {actor_state}");
            assert_eq!(program.duration_ms, duration_ms, "state {actor_state}");
        }
    }
}
