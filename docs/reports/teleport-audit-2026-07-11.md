# Hercules Static Teleport Audit — 2026-07-11

This report preserves every unresolved result from the static teleport audit against the current Korangar `data.grf` and `rdata.grf`.

## Summary

- 5,677 static destinations checked across `Hercules/npc`.
- 371 destinations could not be validated because the destination GAT is missing from the current client archives.
- 58 remaining stock, legacy, instance, or battleground destinations resolve to non-walkable cells and require contextual review.
- The 21 unsafe DM campaign destinations found in the initial pass were corrected separately; the focused campaign audit now has zero unsafe static party warps.
- Dynamic destinations supplied through variables or GM input cannot be proven by this static scan.

Reproduce from `korangar/korangar`:

```bash
cargo run --release --bin map-asset-audit -- \\
  ../../Hercules/conf/map/maps.conf data.grf rdata.grf ../../Hercules/npc
```

## Complete unresolved destination list

Entries ending in `missing gat` are blocked on client assets. Entries with `nearest walkable` have map data but need script-context review before modification.

```text
../../Hercules/npc/quests/okolnir.txt:183: arug_cas04 (321,153): missing gat
../../Hercules/npc/quests/okolnir.txt:184: arug_cas05 (321,153): missing gat
../../Hercules/npc/quests/okolnir.txt:188: schg_cas04 (369,306): missing gat
../../Hercules/npc/quests/okolnir.txt:189: schg_cas05 (369,306): missing gat
../../Hercules/npc/quests/okolnir.txt:985: que_qaru01 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:986: que_qaru02 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:987: que_qaru03 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:988: que_qaru04 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:989: que_qaru05 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:991: que_qsch02 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:992: que_qsch03 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:993: que_qsch04 (139,172): missing gat
../../Hercules/npc/quests/okolnir.txt:994: que_qsch05 (139,172): missing gat
../../Hercules/npc/quests/quests_rachel.txt:6845: que_temsky (99,11): missing gat
../../Hercules/npc/quests/quests_nameless.txt:8611: z_agit (98,40): missing gat
../../Hercules/npc/quests/the_sign_quest.txt:8184: himinn (49,10): missing gat
../../Hercules/npc/quests/the_sign_quest.txt:8194: himinn (49,10): missing gat
../../Hercules/npc/quests/the_sign_quest.txt:9800: que_sign02 (35,313): missing gat
../../Hercules/npc/quests/the_sign_quest.txt:10294: que_sign02 (35,313): missing gat
../../Hercules/npc/quests/dandelion_request.txt:7822: que_job03 (14,182): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:117: new_1-2 (100,70): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:214: new_1-2 (99,99): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:379: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:554: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:577: new_1-2 (84,107): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:596: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:617: new_1-2 (115,107): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:636: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:668: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:837: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:848: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:871: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:894: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:921: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1139: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1159: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1197: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1221: new_1-2 (84,107): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1236: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:1371: new_1-2 (28,178): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2533: new_1-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2597: new_1-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2625: new_1-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2666: new_2-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2669: new_3-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2673: new_1-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2686: new_4-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2689: new_5-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2694: new_2-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2697: new_3-3 (96,21): missing gat
../../Hercules/npc/pre-re/jobs/novice/novice.txt:2730: new_1-4 (99,10): missing gat
../../Hercules/npc/pre-re/warps/other/sign.txt:43: himinn (49,75): missing gat
../../Hercules/npc/pre-re/warps/other/sign.txt:44: himinn (49,63): missing gat
../../Hercules/npc/other/CashShop_Functions.txt:59: random (0,0): missing gat
../../Hercules/npc/other/CashShop_Functions.txt:308: savepoint (0,0): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:336: force_3-1 (40,26): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:345: force_3-1 (25,69): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:354: force_3-1 (25,159): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:363: force_3-1 (69,174): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:372: force_3-1 (159,174): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:381: force_3-1 (174,130): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:390: force_3-1 (174,40): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:399: force_3-1 (132,26): missing gat
../../Hercules/npc/other/arena/arena_lvl70.txt:409: force_3-1 (99,82): missing gat
../../Hercules/npc/other/arena/arena_party.txt:119: force_1-2 (99,26): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:328: force_2-1 (40,26): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:337: force_2-1 (25,69): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:347: force_2-1 (25,159): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:356: force_2-1 (69,174): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:365: force_2-1 (159,174): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:374: force_2-1 (174,130): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:383: force_2-1 (174,40): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:392: force_2-1 (132,26): missing gat
../../Hercules/npc/other/arena/arena_lvl60.txt:402: force_2-1 (99,82): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:318: force_1-1 (40,26): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:327: force_1-1 (25,69): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:337: force_1-1 (25,159): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:346: force_1-1 (69,174): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:355: force_1-1 (159,174): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:364: force_1-1 (174,130): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:373: force_1-1 (174,40): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:382: force_1-1 (132,26): missing gat
../../Hercules/npc/other/arena/arena_lvl50.txt:392: force_1-1 (99,82): missing gat
../../Hercules/npc/other/arena/arena_aco.txt:1052: force_5-1 (40,26): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:339: force_4-1 (40,26): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:348: force_4-1 (25,69): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:357: force_4-1 (25,159): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:366: force_4-1 (69,174): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:376: force_4-1 (159,174): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:385: force_4-1 (174,130): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:394: force_4-1 (174,40): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:403: force_4-1 (132,26): missing gat
../../Hercules/npc/other/arena/arena_lvl80.txt:413: force_4-1 (99,82): missing gat
../../Hercules/npc/other/pvp.txt:440: pvp_c_room (84,39): missing gat
../../Hercules/npc/other/monster_race.txt:1241: p_track02 (75,41): missing gat
../../Hercules/npc/other/monster_race.txt:1264: p_track02 (75,41): missing gat
../../Hercules/npc/other/monster_race.txt:1302: p_track02 (75,41): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:529: bat_a02 (352,342): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:579: bat_a02 (353,52): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:619: bat_a02 (301,209): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:626: bat_a02 (194,261): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:633: bat_a02 (194,270): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:640: bat_a02 (178,125): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:647: bat_a02 (178,134): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:669: bat_a02 (178,228): missing gat
../../Hercules/npc/battleground/tierra/tierra02.txt:677: bat_a02 (200,171): missing gat
../../Hercules/npc/battleground/flavius/flavius02.txt:346: bat_b02 (87,73): missing gat
../../Hercules/npc/battleground/flavius/flavius02.txt:393: bat_b02 (312,225): missing gat
../../Hercules/npc/events/halloween_2008.txt:160: evt_zombie (155,246): missing gat
../../Hercules/npc/events/gdevent_sch.txt:180: schg_que01 (103,133): missing gat
../../Hercules/npc/events/gdevent_sch.txt:213: schg_que01 (103,133): missing gat
../../Hercules/npc/events/gdevent_sch.txt:312: schg_que01 (103,133): missing gat
../../Hercules/npc/events/gdevent_sch.txt:355: schg_que01 (103,133): missing gat
../../Hercules/npc/events/gdevent_sch.txt:454: schg_que01 (103,133): missing gat
../../Hercules/npc/custom/woe_controller.txt:201: savepoint (0,0): missing gat
../../Hercules/npc/custom/etc/morroc_raceway.txt:31: pvp_y_1-5 (165,256): missing gat
../../Hercules/npc/custom/battleground/bg_tierra_02.txt:106: bat_a02 (52,208): missing gat
../../Hercules/npc/custom/battleground/bg_tierra_02.txt:112: bat_a02 (52,208): missing gat
../../Hercules/npc/custom/battleground/bg_tierra_02.txt:424: bat_a02 (52,208): missing gat
../../Hercules/npc/custom/battleground/bg_tierra_02.txt:446: bat_a02 (46,370): missing gat
../../Hercules/npc/custom/battleground/bg_tierra_02.txt:468: bat_a02 (38,12): missing gat
../../Hercules/npc/custom/battleground/bg_flavius_02.txt:368: bat_b02 (382,2): missing gat
../../Hercules/npc/custom/battleground/bg_flavius_02.txt:390: bat_b02 (2,282): missing gat
../../Hercules/npc/warps/guildcastles.txt:431: schg_cas04 (119,8): missing gat
../../Hercules/npc/warps/guildcastles.txt:432: schg_cas04 (120,7): missing gat
../../Hercules/npc/warps/guildcastles.txt:434: schg_cas05 (119,8): missing gat
../../Hercules/npc/warps/guildcastles.txt:435: schg_cas05 (120,7): missing gat
../../Hercules/npc/warps/guildcastles.txt:452: arug_cas04 (141,45): missing gat
../../Hercules/npc/warps/guildcastles.txt:453: arug_cas04 (141,45): missing gat
../../Hercules/npc/warps/guildcastles.txt:455: arug_cas05 (141,45): missing gat
../../Hercules/npc/warps/guildcastles.txt:456: arug_cas05 (141,45): missing gat
../../Hercules/npc/warps/guildcastles.txt:460: nguild_alde (34,248): missing gat
../../Hercules/npc/warps/guildcastles.txt:463: nguild_alde (104,108): missing gat
../../Hercules/npc/warps/guildcastles.txt:464: nguild_alde (45,224): missing gat
../../Hercules/npc/warps/guildcastles.txt:465: nguild_alde (122,61): missing gat
../../Hercules/npc/warps/guildcastles.txt:466: nguild_alde (62,191): missing gat
../../Hercules/npc/warps/guildcastles.txt:467: nguild_alde (62,191): missing gat
../../Hercules/npc/warps/guildcastles.txt:468: nguild_alde (50,70): missing gat
../../Hercules/npc/warps/guildcastles.txt:469: nguild_alde (24,188): missing gat
../../Hercules/npc/warps/guildcastles.txt:470: nguild_alde (42,225): missing gat
../../Hercules/npc/warps/guildcastles.txt:471: nguild_alde (70,108): missing gat
../../Hercules/npc/warps/guildcastles.txt:472: nguild_alde (207,132): missing gat
../../Hercules/npc/warps/guildcastles.txt:473: nguild_alde (89,27): missing gat
../../Hercules/npc/warps/guildcastles.txt:474: nguild_alde (216,50): missing gat
../../Hercules/npc/warps/guildcastles.txt:475: nguild_alde (206,184): missing gat
../../Hercules/npc/warps/guildcastles.txt:476: nguild_alde (42,197): missing gat
../../Hercules/npc/warps/guildcastles.txt:477: nguild_alde (232,182): missing gat
../../Hercules/npc/warps/guildcastles.txt:478: nguild_alde (35,197): missing gat
../../Hercules/npc/warps/guildcastles.txt:479: nguild_alde (175,175): missing gat
../../Hercules/npc/warps/guildcastles.txt:482: nguild_gef (34,140): missing gat
../../Hercules/npc/warps/guildcastles.txt:486: nguild_gef (50,84): missing gat
../../Hercules/npc/warps/guildcastles.txt:487: nguild_gef (30,167): missing gat
../../Hercules/npc/warps/guildcastles.txt:488: nguild_gef (198,160): missing gat
../../Hercules/npc/warps/guildcastles.txt:489: nguild_gef (185,52): missing gat
../../Hercules/npc/warps/guildcastles.txt:490: nguild_gef (56,170): missing gat
../../Hercules/npc/warps/guildcastles.txt:491: nguild_gef (33,47): missing gat
../../Hercules/npc/warps/guildcastles.txt:492: nguild_gef (35,185): missing gat
../../Hercules/npc/warps/guildcastles.txt:493: nguild_gef (174,34): missing gat
../../Hercules/npc/warps/guildcastles.txt:494: nguild_gef (62,13): missing gat
../../Hercules/npc/warps/guildcastles.txt:495: nguild_gef (174,14): missing gat
../../Hercules/npc/warps/guildcastles.txt:496: nguild_gef (90,47): missing gat
../../Hercules/npc/warps/guildcastles.txt:497: nguild_gef (205,34): missing gat
../../Hercules/npc/warps/guildcastles.txt:498: nguild_gef (39,192): missing gat
../../Hercules/npc/warps/guildcastles.txt:499: nguild_gef (54,185): missing gat
../../Hercules/npc/warps/guildcastles.txt:502: nguild_pay (214,48): missing gat
../../Hercules/npc/warps/guildcastles.txt:505: nguild_pay (102,19): missing gat
../../Hercules/npc/warps/guildcastles.txt:506: nguild_pay (201,122): missing gat
../../Hercules/npc/warps/guildcastles.txt:507: nguild_pay (130,43): missing gat
../../Hercules/npc/warps/guildcastles.txt:508: nguild_pay (226,130): missing gat
../../Hercules/npc/warps/guildcastles.txt:509: nguild_pay (230,94): missing gat
../../Hercules/npc/warps/guildcastles.txt:510: nguild_pay (222,112): missing gat
../../Hercules/npc/warps/guildcastles.txt:511: nguild_pay (201,118): missing gat
../../Hercules/npc/warps/guildcastles.txt:512: nguild_pay (213,72): missing gat
../../Hercules/npc/warps/guildcastles.txt:513: nguild_pay (15,115): missing gat
../../Hercules/npc/warps/guildcastles.txt:514: nguild_pay (81,15): missing gat
../../Hercules/npc/warps/guildcastles.txt:515: nguild_pay (115,147): missing gat
../../Hercules/npc/warps/guildcastles.txt:516: nguild_pay (53,115): missing gat
../../Hercules/npc/warps/guildcastles.txt:519: nguild_prt (99,32): missing gat
../../Hercules/npc/warps/guildcastles.txt:522: nguild_prt (202,183): missing gat
../../Hercules/npc/warps/guildcastles.txt:523: nguild_prt (75,187): missing gat
../../Hercules/npc/warps/guildcastles.txt:524: nguild_prt (40,54): missing gat
../../Hercules/npc/warps/guildcastles.txt:525: nguild_prt (75,54): missing gat
../../Hercules/npc/warps/guildcastles.txt:526: nguild_prt (113,163): missing gat
../../Hercules/npc/warps/guildcastles.txt:527: nguild_prt (55,70): missing gat
../../Hercules/npc/warps/guildcastles.txt:528: nguild_prt (45,34): missing gat
../../Hercules/npc/warps/guildcastles.txt:529: nguild_prt (192,119): missing gat
../../Hercules/npc/warps/guildcastles.txt:530: nguild_prt (40,47): missing gat
../../Hercules/npc/warps/guildcastles.txt:531: nguild_prt (202,92): missing gat
../../Hercules/npc/warps/guildcastles.txt:532: nguild_prt (80,49): missing gat
../../Hercules/npc/warps/guildcastles.txt:533: nguild_prt (192,119): missing gat
../../Hercules/npc/warps/guildcastles.txt:534: nguild_prt (192,65): missing gat
../../Hercules/npc/warps/guildcastles.txt:535: nguild_prt (147,116): missing gat
../../Hercules/npc/warps/guildcastles.txt:536: nguild_prt (192,65): missing gat
../../Hercules/npc/warps/guildcastles.txt:537: nguild_prt (61,19): missing gat
../../Hercules/npc/warps/other/jobquests.txt:40: new_1-2 (100,9): missing gat
../../Hercules/npc/warps/other/jobquests.txt:41: new_2-2 (100,9): missing gat
../../Hercules/npc/warps/other/jobquests.txt:42: new_3-2 (100,9): missing gat
../../Hercules/npc/warps/other/jobquests.txt:43: new_4-2 (100,9): missing gat
../../Hercules/npc/warps/other/jobquests.txt:44: new_5-2 (100,9): missing gat
../../Hercules/npc/warps/other/jobquests.txt:45: new_1-1 (144,112): missing gat
../../Hercules/npc/warps/other/jobquests.txt:46: new_2-1 (144,112): missing gat
../../Hercules/npc/warps/other/jobquests.txt:47: new_3-1 (144,112): missing gat
../../Hercules/npc/warps/other/jobquests.txt:48: new_4-1 (144,112): missing gat
../../Hercules/npc/warps/other/jobquests.txt:49: new_5-1 (144,112): missing gat
../../Hercules/npc/warps/other/jobquests.txt:50: new_1-2 (160,171): missing gat
../../Hercules/npc/warps/other/jobquests.txt:51: new_2-2 (160,171): missing gat
../../Hercules/npc/warps/other/jobquests.txt:52: new_3-2 (160,171): missing gat
../../Hercules/npc/warps/other/jobquests.txt:53: new_4-2 (160,171): missing gat
../../Hercules/npc/warps/other/jobquests.txt:54: new_5-2 (160,171): missing gat
../../Hercules/npc/warps/other/jobquests.txt:55: new_1-2 (123,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:56: new_2-2 (123,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:57: new_3-2 (123,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:58: new_4-2 (123,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:59: new_5-2 (123,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:60: new_1-2 (41,172): missing gat
../../Hercules/npc/warps/other/jobquests.txt:61: new_2-2 (41,172): missing gat
../../Hercules/npc/warps/other/jobquests.txt:62: new_3-2 (41,172): missing gat
../../Hercules/npc/warps/other/jobquests.txt:63: new_4-2 (41,172): missing gat
../../Hercules/npc/warps/other/jobquests.txt:64: new_5-2 (41,172): missing gat
../../Hercules/npc/warps/other/jobquests.txt:65: new_1-2 (78,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:66: new_2-2 (78,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:67: new_3-2 (78,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:68: new_4-2 (78,106): missing gat
../../Hercules/npc/warps/other/jobquests.txt:69: new_5-2 (78,106): missing gat
../../Hercules/npc/warps/other/airplane.txt:55: lhz_airport (19,20): missing gat
../../Hercules/npc/warps/other/airplane.txt:56: lhz_airport (123,14): missing gat
../../Hercules/npc/warps/other/airplane.txt:57: lhz_airport (48,20): missing gat
../../Hercules/npc/warps/other/airplane.txt:58: lhz_airport (162,14): missing gat
../../Hercules/npc/warps/other/airplane.txt:59: lhz_airport (143,15): missing gat
../../Hercules/npc/warps/other/airplane.txt:62: lhz_airport (143,53): missing gat
../../Hercules/npc/warps/other/airplane.txt:67: y_airport (19,20): missing gat
../../Hercules/npc/warps/other/airplane.txt:68: y_airport (123,14): missing gat
../../Hercules/npc/warps/other/airplane.txt:69: y_airport (48,20): missing gat
../../Hercules/npc/warps/other/airplane.txt:70: y_airport (162,14): missing gat
../../Hercules/npc/warps/other/airplane.txt:71: y_airport (143,23): missing gat
../../Hercules/npc/warps/other/airplane.txt:73: y_airport (143,54): missing gat
../../Hercules/npc/warps/other/airplane.txt:74: y_airport (143,54): missing gat
../../Hercules/npc/warps/other/airplane.txt:75: airplane_01 (244,58): missing gat
../../Hercules/npc/warps/other/airplane.txt:85: airplane_01 (91,67): missing gat
../../Hercules/npc/warps/other/airplane.txt:86: airplane_01 (250,54): missing gat
../../Hercules/npc/warps/other/airplane.txt:87: airplane_01 (239,160): missing gat
../../Hercules/npc/warps/other/airplane.txt:88: airplane_01 (214,54): missing gat
../../Hercules/npc/warps/other/airplane.txt:89: airplane_01 (105,72): missing gat
../../Hercules/npc/warps/other/airplane.txt:90: airplane_01 (102,199): missing gat
../../Hercules/npc/warps/other/airplane.txt:91: airplane_01 (105,52): missing gat
../../Hercules/npc/warps/other/airplane.txt:92: airplane_01 (102,176): missing gat
../../Hercules/npc/warps/other/arena.txt:55: force_5-1 (25,69): missing gat
../../Hercules/npc/warps/other/arena.txt:56: force_5-1 (25,159): missing gat
../../Hercules/npc/warps/other/arena.txt:57: force_5-1 (69,174): missing gat
../../Hercules/npc/warps/other/arena.txt:58: force_5-1 (159,174): missing gat
../../Hercules/npc/warps/other/arena.txt:59: force_5-1 (174,130): missing gat
../../Hercules/npc/warps/other/arena.txt:60: force_5-1 (174,40): missing gat
../../Hercules/npc/warps/other/arena.txt:61: force_5-1 (132,26): missing gat
../../Hercules/npc/warps/other/arena.txt:62: force_5-1 (99,82): missing gat
../../Hercules/npc/warps/other/arena.txt:66: force_1-2 (37,26): missing gat
../../Hercules/npc/warps/other/arena.txt:67: force_1-2 (162,26): missing gat
../../Hercules/npc/warps/other/arena.txt:68: force_1-2 (99,66): missing gat
../../Hercules/npc/warps/other/arena.txt:69: force_1-2 (89,26): missing gat
../../Hercules/npc/warps/other/arena.txt:70: force_1-2 (110,26): missing gat
../../Hercules/npc/warps/other/arena.txt:71: force_1-2 (99,36): missing gat
../../Hercules/npc/warps/other/arena.txt:72: force_1-2 (37,78): missing gat
../../Hercules/npc/warps/other/arena.txt:73: force_1-2 (162,78): missing gat
../../Hercules/npc/warps/other/arena.txt:74: force_1-2 (110,78): missing gat
../../Hercules/npc/warps/other/arena.txt:75: force_1-2 (37,78): missing gat
../../Hercules/npc/warps/other/arena.txt:76: force_1-2 (26,118): missing gat
../../Hercules/npc/warps/other/arena.txt:77: force_1-2 (91,125): missing gat
../../Hercules/npc/warps/other/arena.txt:78: force_1-2 (173,118): missing gat
../../Hercules/npc/warps/other/arena.txt:79: force_1-2 (133,178): missing gat
../../Hercules/npc/warps/other/arena.txt:80: force_1-2 (29,178): missing gat
../../Hercules/npc/warps/other/arena.txt:81: force_1-2 (59,178): missing gat
../../Hercules/npc/warps/pvp.txt:38: ordeal_1-1 (128,150): missing gat
../../Hercules/npc/warps/pvp.txt:39: ordeal_1-1 (95,150): missing gat
../../Hercules/npc/warps/pvp.txt:40: ordeal_1-1 (135,163): missing gat
../../Hercules/npc/warps/pvp.txt:41: ordeal_1-1 (109,188): missing gat
../../Hercules/npc/warps/pvp.txt:42: ordeal_1-1 (136,136): missing gat
../../Hercules/npc/warps/pvp.txt:43: ordeal_1-1 (110,110): missing gat
../../Hercules/npc/warps/pvp.txt:44: ordeal_1-1 (149,204): missing gat
../../Hercules/npc/warps/pvp.txt:45: ordeal_1-1 (148,171): missing gat
../../Hercules/npc/warps/pvp.txt:46: ordeal_1-1 (151,129): missing gat
../../Hercules/npc/warps/pvp.txt:47: ordeal_1-1 (151,94): missing gat
../../Hercules/npc/warps/pvp.txt:48: ordeal_1-1 (189,189): missing gat
../../Hercules/npc/warps/pvp.txt:49: ordeal_1-1 (163,163): missing gat
../../Hercules/npc/warps/pvp.txt:50: ordeal_1-1 (188,111): missing gat
../../Hercules/npc/warps/pvp.txt:51: ordeal_1-1 (164,136): missing gat
../../Hercules/npc/warps/pvp.txt:52: ordeal_1-1 (204,150): missing gat
../../Hercules/npc/warps/pvp.txt:53: ordeal_1-1 (171,150): missing gat
../../Hercules/npc/warps/pvp.txt:54: ordeal_1-2 (24,154): missing gat
../../Hercules/npc/warps/pvp.txt:55: ordeal_1-2 (24,24): missing gat
../../Hercules/npc/warps/pvp.txt:56: ordeal_1-2 (24,284): missing gat
../../Hercules/npc/warps/pvp.txt:57: ordeal_1-2 (153,23): missing gat
../../Hercules/npc/warps/pvp.txt:58: ordeal_1-2 (144,284): missing gat
../../Hercules/npc/warps/pvp.txt:59: ordeal_1-2 (284,24): missing gat
../../Hercules/npc/warps/pvp.txt:60: ordeal_1-2 (284,284): missing gat
../../Hercules/npc/warps/pvp.txt:61: ordeal_1-2 (284,164): missing gat
../../Hercules/npc/warps/pvp.txt:86: ordeal_2-1 (128,150): missing gat
../../Hercules/npc/warps/pvp.txt:87: ordeal_2-1 (95,150): missing gat
../../Hercules/npc/warps/pvp.txt:88: ordeal_2-1 (135,163): missing gat
../../Hercules/npc/warps/pvp.txt:89: ordeal_2-1 (109,188): missing gat
../../Hercules/npc/warps/pvp.txt:90: ordeal_2-1 (136,136): missing gat
../../Hercules/npc/warps/pvp.txt:91: ordeal_2-1 (110,110): missing gat
../../Hercules/npc/warps/pvp.txt:92: ordeal_2-1 (149,204): missing gat
../../Hercules/npc/warps/pvp.txt:93: ordeal_2-1 (148,171): missing gat
../../Hercules/npc/warps/pvp.txt:94: ordeal_2-1 (151,129): missing gat
../../Hercules/npc/warps/pvp.txt:95: ordeal_2-1 (151,94): missing gat
../../Hercules/npc/warps/pvp.txt:96: ordeal_2-1 (189,189): missing gat
../../Hercules/npc/warps/pvp.txt:97: ordeal_2-1 (163,163): missing gat
../../Hercules/npc/warps/pvp.txt:98: ordeal_2-1 (188,111): missing gat
../../Hercules/npc/warps/pvp.txt:99: ordeal_2-1 (164,136): missing gat
../../Hercules/npc/warps/pvp.txt:100: ordeal_2-1 (204,150): missing gat
../../Hercules/npc/warps/pvp.txt:101: ordeal_2-1 (171,150): missing gat
../../Hercules/npc/warps/pvp.txt:102: ordeal_2-2 (24,154): missing gat
../../Hercules/npc/warps/pvp.txt:103: ordeal_2-2 (24,24): missing gat
../../Hercules/npc/warps/pvp.txt:104: ordeal_2-2 (24,284): missing gat
../../Hercules/npc/warps/pvp.txt:105: ordeal_2-2 (153,23): missing gat
../../Hercules/npc/warps/pvp.txt:106: ordeal_2-2 (144,284): missing gat
../../Hercules/npc/warps/pvp.txt:107: ordeal_2-2 (284,24): missing gat
../../Hercules/npc/warps/pvp.txt:108: ordeal_2-2 (284,284): missing gat
../../Hercules/npc/warps/pvp.txt:109: ordeal_2-2 (284,164): missing gat
../../Hercules/npc/warps/pvp.txt:134: ordeal_3-1 (128,150): missing gat
../../Hercules/npc/warps/pvp.txt:135: ordeal_3-1 (95,150): missing gat
../../Hercules/npc/warps/pvp.txt:136: ordeal_3-1 (135,163): missing gat
../../Hercules/npc/warps/pvp.txt:137: ordeal_3-1 (109,188): missing gat
../../Hercules/npc/warps/pvp.txt:138: ordeal_3-1 (136,136): missing gat
../../Hercules/npc/warps/pvp.txt:139: ordeal_3-1 (110,110): missing gat
../../Hercules/npc/warps/pvp.txt:140: ordeal_3-1 (149,204): missing gat
../../Hercules/npc/warps/pvp.txt:141: ordeal_3-1 (148,171): missing gat
../../Hercules/npc/warps/pvp.txt:142: ordeal_3-1 (151,129): missing gat
../../Hercules/npc/warps/pvp.txt:143: ordeal_3-1 (151,94): missing gat
../../Hercules/npc/warps/pvp.txt:144: ordeal_3-1 (189,189): missing gat
../../Hercules/npc/warps/pvp.txt:145: ordeal_3-1 (163,163): missing gat
../../Hercules/npc/warps/pvp.txt:146: ordeal_3-1 (188,111): missing gat
../../Hercules/npc/warps/pvp.txt:147: ordeal_3-1 (164,136): missing gat
../../Hercules/npc/warps/pvp.txt:148: ordeal_3-1 (204,150): missing gat
../../Hercules/npc/warps/pvp.txt:149: ordeal_3-1 (171,150): missing gat
../../Hercules/npc/warps/pvp.txt:150: ordeal_3-2 (24,154): missing gat
../../Hercules/npc/warps/pvp.txt:151: ordeal_3-2 (24,24): missing gat
../../Hercules/npc/warps/pvp.txt:152: ordeal_3-2 (24,284): missing gat
../../Hercules/npc/warps/pvp.txt:153: ordeal_3-2 (153,23): missing gat
../../Hercules/npc/warps/pvp.txt:154: ordeal_3-2 (144,284): missing gat
../../Hercules/npc/warps/pvp.txt:155: ordeal_3-2 (284,24): missing gat
../../Hercules/npc/warps/pvp.txt:156: ordeal_3-2 (284,284): missing gat
../../Hercules/npc/warps/pvp.txt:157: ordeal_3-2 (284,164): missing gat
../../Hercules/npc/warps/pvp.txt:184: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:185: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:186: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:187: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:188: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:189: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:190: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/warps/pvp.txt:191: pvp_n_8-4 (0,0): missing gat
../../Hercules/npc/re/quests/eden/eden_iro.txt:924: auction_03 (151,23): missing gat
../../Hercules/npc/re/quests/quests_dicastes.txt:304: dic_dun03 (101,142): missing gat
../../Hercules/npc/re/instances/WolfchevLaboratory.txt:1942: 1@lhz.gat (45,148): missing gat
../../Hercules/npc/re/jobs/3-2/sura.txt:496: sword_1-1 (215,244): missing gat
../../Hercules/npc/re/jobs/3-2/sura.txt:508: sword_1-1 (216,168): missing gat
../../Hercules/npc/re/jobs/3-2/sura.txt:526: sword_1-1 (215,244): missing gat
../../Hercules/npc/re/jobs/3-2/sura.txt:672: sword_1-1 (216,168): missing gat
../../Hercules/npc/re/jobs/3-2/sura.txt:691: sword_1-1 (216,168): missing gat
../../Hercules/npc/re/jobs/3-1/mechanic.txt:357: jupe_core2 (149,288): missing gat
../../Hercules/npc/re/warps/other/sign.txt:43: himinn (49,75): missing gat
../../Hercules/npc/re/warps/other/sign.txt:44: himinn (49,63): missing gat
../../Hercules/npc/airports/lighthalzen.txt:51: lhz_airport (148,51): missing gat
../../Hercules/npc/airports/lighthalzen.txt:56: lhz_airport (148,51): missing gat
../../Hercules/npc/airports/lighthalzen.txt:91: lhz_airport (142,40): missing gat
../../Hercules/npc/airports/yuno.txt:50: y_airport (148,51): missing gat
../../Hercules/npc/airports/yuno.txt:55: y_airport (148,51): missing gat
../../Hercules/npc/airports/yuno.txt:88: y_airport (142,40): missing gat
../../Hercules/npc/airports/izlude.txt:52: airplane_01 (244,58): missing gat
../../Hercules/npc/airports/izlude.txt:57: airplane_01 (244,58): missing gat
../../Hercules/npc/airports/rachel.txt:45: airplane_01 (245,60): missing gat
../../Hercules/npc/airports/rachel.txt:50: airplane_01 (245,60): missing gat
../../Hercules/npc/quests/quests_lighthalzen.txt:4320: lhz_in01 (278,162), nearest walkable: Some((277, 161))
../../Hercules/npc/quests/quests_lighthalzen.txt:4323: lhz_in01 (278,162), nearest walkable: Some((277, 161))
../../Hercules/npc/quests/quests_lighthalzen.txt:5011: lighthalzen (322,323), nearest walkable: Some((321, 322))
../../Hercules/npc/quests/seals/brisingamen_seal.txt:3025: que_god02 (15,125), nearest walkable: Some((18, 125))
../../Hercules/npc/quests/seals/brisingamen_seal.txt:3046: que_god02 (15,125), nearest walkable: Some((18, 125))
../../Hercules/npc/quests/quests_nameless.txt:8746: moc_fild17 (209,235), nearest walkable: Some((208, 234))
../../Hercules/npc/quests/quests_nameless.txt:8775: moc_fild17 (209,235), nearest walkable: Some((208, 234))
../../Hercules/npc/quests/quests_nameless.txt:9036: moc_fild17 (209,235), nearest walkable: Some((208, 234))
../../Hercules/npc/quests/quests_nameless.txt:9160: moc_fild17 (209,235), nearest walkable: Some((208, 234))
../../Hercules/npc/quests/quests_nameless.txt:9280: moc_fild17 (209,235), nearest walkable: Some((208, 234))
../../Hercules/npc/quests/the_sign_quest.txt:12262: geffen (116,115), nearest walkable: Some((114, 113))
../../Hercules/npc/quests/first_class/tu_sword.txt:1097: izlude (35,78), nearest walkable: Some((44, 89))
../../Hercules/npc/quests/first_class/tu_sword.txt:2208: izlude (35,78), nearest walkable: Some((44, 89))
../../Hercules/npc/pre-re/warps/cities/izlude.txt:45: izlude (212,129), nearest walkable: Some((204, 146))
../../Hercules/npc/pre-re/warps/cities/izlude.txt:52: izlude (52,136), nearest walkable: Some((54, 152))
../../Hercules/npc/pre-re/warps/cities/izlude.txt:54: izlude (182,56), nearest walkable: Some((180, 54))
../../Hercules/npc/pre-re/warps/cities/izlude.txt:55: izlude (145,40), nearest walkable: Some((135, 49))
../../Hercules/npc/pre-re/warps/fields/prontera_fild.txt:88: izlude (35,78), nearest walkable: Some((44, 89))
../../Hercules/npc/pre-re/warps/fields/hugel_fild.txt:49: ein_fild01 (231,40), nearest walkable: Some((163, 107))
../../Hercules/npc/woe-fe/prtg_cas01.txt:129: prtg_cas01 (112,183), nearest walkable: Some((111, 182))
../../Hercules/npc/other/hugel_bingo.txt:170: que_bingo (44,115), nearest walkable: Some((43, 114))
../../Hercules/npc/other/hugel_bingo.txt:175: que_bingo (44,115), nearest walkable: Some((43, 114))
../../Hercules/npc/other/hugel_bingo.txt:288: que_bingo (44,115), nearest walkable: Some((43, 114))
../../Hercules/npc/other/hugel_bingo.txt:729: que_bingo (44,115), nearest walkable: Some((43, 114))
../../Hercules/npc/other/powernpc.txt:245: gon_test (41,81), nearest walkable: Some((42, 84))
../../Hercules/npc/cities/alberta.txt:240: izlude (176,182), nearest walkable: Some((170, 176))
../../Hercules/npc/cities/jawaii.txt:66: izlude (176,182), nearest walkable: Some((170, 176))
../../Hercules/npc/cities/louyang.txt:321: lou_in01 (17,19), nearest walkable: Some((16, 18))
../../Hercules/npc/cities/ayothaya.txt:120: alberta (238,22), nearest walkable: Some((234, 26))
../../Hercules/npc/cities/izlude.txt:520: izlude (176,182), nearest walkable: Some((170, 176))
../../Hercules/npc/cities/comodo.txt:378: izlude (176,182), nearest walkable: Some((170, 176))
../../Hercules/npc/jobs/2-1/assassin.txt:1339: in_moc_16 (60,136), nearest walkable: Some((62, 148))
../../Hercules/npc/jobs/2-2/sage.txt:2673: job_sage (100,82), nearest walkable: Some((103, 82))
../../Hercules/npc/jobs/2-2/sage.txt:2776: job_sage (100,82), nearest walkable: Some((103, 82))
../../Hercules/npc/jobs/2-2/sage.txt:2842: job_sage (100,82), nearest walkable: Some((103, 82))
../../Hercules/npc/jobs/2-2/sage.txt:2891: job_sage (100,82), nearest walkable: Some((103, 82))
../../Hercules/npc/jobs/2-2/crusader.txt:1455: job_cru (160,14), nearest walkable: Some((162, 16))
../../Hercules/npc/events/gdevent_sch.txt:1438: schg_dun01 (199,192), nearest walkable: Some((189, 202))
../../Hercules/npc/custom/etc/penal_servitude.txt:76: izlude (105,112), nearest walkable: Some((102, 115))
../../Hercules/npc/custom/etc/penal_servitude.txt:152: izlude (105,112), nearest walkable: Some((102, 115))
../../Hercules/npc/custom/battleground/bg_flavius_01.txt:368: bat_b01 (382,2), nearest walkable: Some((384, 4))
../../Hercules/npc/custom/battleground/bg_flavius_01.txt:390: bat_b01 (2,282), nearest walkable: Some((6, 282))
../../Hercules/npc/re/quests/quests_malangdo.txt:1350: mal_dun01 (0,0), nearest walkable: Some((47, 50))
../../Hercules/npc/re/quests/quests_malaya.txt:2426: izlude (195,180), nearest walkable: Some((199, 174))
../../Hercules/npc/re/instances/OldGlastHeim.txt:736: 1@gl_k (269,264), nearest walkable: Some((268, 263))
../../Hercules/npc/re/instances/HazyForest.txt:1088: bif_fild01 (160,352), nearest walkable: Some((161, 351))
../../Hercules/npc/re/instances/HazyForest.txt:1107: 1@mist (141,90), nearest walkable: Some((140, 89))
../../Hercules/npc/re/warps/cities/izlude.txt:170: iz_ac02 (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:171: iz_ac02 (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:189: iz_ac02_a (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:190: iz_ac02_b (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:191: iz_ac02_c (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:192: iz_ac02_d (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:193: iz_ac02_a (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:194: iz_ac02_b (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:195: iz_ac02_c (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/re/warps/cities/izlude.txt:196: iz_ac02_d (104,27), nearest walkable: Some((124, 46))
../../Hercules/npc/airports/airships.txt:678: izlude (200,56), nearest walkable: Some((190, 65))
```

