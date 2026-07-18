use ragnarok_packets::SkillId;

/// No-action sentinel returned by Ragexe `0x009B4B30` for skills whose
/// presentation must not replace the actor's current action.
pub(super) const NO_ACTION: i8 = -1;

/// Exhaustive skill-ID to actor-state lookup recovered from the reference
/// Ragexe `0x009B4B30`. Every unlisted unsigned 16-bit skill ID returns the
/// native default state 7.
pub(super) fn actor_state(skill_id: SkillId) -> i8 {
    match skill_id.0 {
        29
        | 151
        | 196
        | 209
        | 249
        | 252
        | 256..=257
        | 261
        | 268..=269
        | 304
        | 349..=350
        | 356
        | 369
        | 380
        | 384
        | 387
        | 401
        | 411
        | 444
        | 690
        | 730
        | 1015
        | 2334
        | 8204
        | 8219..=8220 => 0,
        364 => 1,
        5
        | 7
        | 42
        | 46..=47
        | 56..=58
        | 61..=62
        | 109
        | 212
        | 214
        | 219
        | 253
        | 263
        | 266..=267
        | 338
        | 366..=368
        | 370
        | 372
        | 379
        | 382
        | 397..=399
        | 406
        | 484
        | 499
        | 502..=503
        | 512..=515
        | 518..=520
        | 661
        | 674
        | 712
        | 728
        | 732
        | 740
        | 1001
        | 1005
        | 1009
        | 1016
        | 2002
        | 2004..=2006
        | 2017
        | 2022..=2023
        | 2029..=2031
        | 2036..=2037
        | 2233
        | 2236
        | 2259
        | 2261
        | 2279..=2280
        | 2284
        | 2288
        | 2298
        | 2314
        | 2317
        | 2320
        | 2324
        | 2342
        | 2476 => 2,
        115..=125 | 688 | 1013 | 2238..=2239 | 2249..=2254 | 2267 => 5,
        316 | 324 | 394 | 530 | 2258 | 2307..=2308 | 2312 | 3004 => 9,
        707 | 2003 | 2011 => 11,
        59 | 149 | 152 | 229..=232 | 250..=251 | 480 | 1004 | 2278 | 2480 | 2493 => 12,
        306..=313 | 325 | 327..=330 | 395..=396 | 488 | 1011 => 16,
        317 | 319..=322 => 17,
        2327 | 2330 => 18,
        426 | 2581 | 5023 => 20,
        412 | 2015 | 2285 => 21,
        414 | 2478 => 22,
        416 | 445 | 447..=458 | 460..=461 | 494 | 572..=575 | 2333 | 2340 | 2596..=2599 => 23,
        418 | 2018 | 2343 => 24,
        420 | 2269 | 2323 => 25,
        419 => 26,
        421 | 2336 => 27,
        413 | 2337 | 2576 => 30,
        415 | 2313 => 31,
        417 | 2311 | 2344..=2348 | 2593 => 32,
        431..=433 => 33,
        428..=430 => 34,
        527..=529 | 532 | 535..=538 => 35,
        523..=526 | 531 | 534 | 539..=543 | 3005..=3009 | 3020 | 3022..=3023 => 36,
        3011 | 3013..=3018 | 3021 | 3026..=3027 | 3029 => 37,
        501 | 504..=506 | 516..=517 => 38,
        500 | 2256..=2257 | 2260 | 2271..=2272 | 2445 => 39,
        508 | 521 => 40,
        2268 | 2273..=2275 => 43,
        2270 => 44,
        5021 => 49,
        2580 => 51,
        446
        | 478
        | 490
        | 2028
        | 2215
        | 2218..=2221
        | 2240..=2242
        | 2263..=2265
        | 2415..=2416
        | 2481..=2482
        | 2485..=2486
        | 2490
        | 2516
        | 2520 => NO_ACTION,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ragnarok_packets::SkillId;

    use super::{NO_ACTION, actor_state};

    #[test]
    fn representative_skill_from_every_native_row_resolves_exactly() {
        for (skill_id, expected_state) in [
            (29, 0),
            (364, 1),
            (5, 2),
            (115, 5),
            (316, 9),
            (707, 11),
            (59, 12),
            (306, 16),
            (317, 17),
            (2327, 18),
            (426, 20),
            (412, 21),
            (414, 22),
            (416, 23),
            (418, 24),
            (420, 25),
            (419, 26),
            (421, 27),
            (413, 30),
            (415, 31),
            (417, 32),
            (431, 33),
            (428, 34),
            (527, 35),
            (523, 36),
            (3011, 37),
            (501, 38),
            (500, 39),
            (508, 40),
            (2268, 43),
            (2270, 44),
            (5021, 49),
            (2580, 51),
            (446, NO_ACTION),
        ] {
            assert_eq!(actor_state(SkillId(skill_id)), expected_state, "skill {skill_id}");
        }
    }

    #[test]
    fn default_and_range_boundaries_do_not_bleed() {
        for skill_id in [0, 4, 6, 8, 114, 126, 305, 314, 65535] {
            assert_eq!(actor_state(SkillId(skill_id)), 7, "skill {skill_id}");
        }
        assert_eq!(actor_state(SkillId(8218)), 7);
        assert_eq!(actor_state(SkillId(8219)), 0);
        assert_eq!(actor_state(SkillId(8220)), 0);
        assert_eq!(actor_state(SkillId(8221)), 7);
    }

    #[test]
    fn full_u16_distribution_matches_reference_executable_audit() {
        let mut distribution = BTreeMap::new();
        for skill_id in u16::MIN..=u16::MAX {
            *distribution.entry(actor_state(SkillId(skill_id))).or_insert(0usize) += 1;
        }

        let expected = BTreeMap::from([
            (-1, 24),
            (0, 29),
            (1, 1),
            (2, 78),
            (5, 22),
            (7, 65209),
            (9, 9),
            (11, 3),
            (12, 14),
            (16, 17),
            (17, 5),
            (18, 2),
            (20, 3),
            (21, 3),
            (22, 2),
            (23, 27),
            (24, 3),
            (25, 3),
            (26, 1),
            (27, 2),
            (30, 3),
            (31, 2),
            (32, 8),
            (33, 3),
            (34, 3),
            (35, 8),
            (36, 19),
            (37, 11),
            (38, 6),
            (39, 7),
            (40, 2),
            (43, 4),
            (44, 1),
            (49, 1),
            (51, 1),
        ]);
        assert_eq!(distribution, expected);
    }
}
