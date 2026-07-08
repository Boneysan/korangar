# Cards in the Game - Complete Table

**Source:** Hercules_RO `db/re/item_db.conf` (cards) + `db/re/mob_db.conf` (drops)

**Summary:** 1012 card items (IT_CARD type, IDs 4001+).

Columns:
- ID: Item ID
- Name: Display name (cleaned)
- Location: Equipment slot it compounds to
- Effect: Bonus/effect script
- Dropped By: Monsters (SpriteName) and base chance (1 = very rare)

Use alongside the Bestiary for full monster+card ecosystem in DM campaign.

| ID | Name | Location | Effect | Dropped By |
| --- | --- | --- | --- | --- |
| 4001 | Poring Card | EQP_ARMOR | bonus bLuk,2; bonus bFlee2,1; | PORING:1, C3_PORING:1, C4_PORING:1 +2 |
| 4002 | Fabre Card | EQP_WEAPON | bonus bVit,1; bonus bMaxHP,100; | FABRE:1, META_FABRE:1, C2_FABRE:1 +1 |
| 4003 | Pupa Card | EQP_ARMOR | bonus bMaxHP,700; | PUPA:1, META_PUPA:1 |
| 4004 | Drops Card | EQP_WEAPON | bonus bDex,1; bonus bHit,3; | DROPS:1, E_DROPS:1, C3_DROPS:1 |
| 4005 | Poring  Card | EQP_WEAPON | bonus2 bAddEle,Ele_Dark,20; | PORING_:1, PORING_V:100 |
| 4006 | Lunatic Card | EQP_WEAPON | bonus bLuk,1; bonus bCritical,1; bonus bFlee2,1; | LUNATIC:1, C2_LUNATIC:1, C3_LUNATIC:1 |
| 4007 | Pecopeco Egg Card | EQP_WEAPON | bonus2 bAddRace,RC_Formless,20; | PECOPECO_EGG:1, META_PECOPECO_EGG:1 |
| 4008 | Picky Card | EQP_ARMOR | bonus bStr,1; bonus bBaseAtk,10; | PICKY:1, META_PICKY:1 |
| 4009 | Chonchon Card | EQP_SHOES | bonus bAgi,1; bonus bFlee,2; | CHONCHON:1, CHONCHON_:1, C5_CHONCHON:1 |
| 4010 | Wilow Card | EQP_HEAD_LOW | bonus bMaxSP,80; | WILOW:1, C3_WILOW:1 |
| 4011 | Picky  Card | EQP_ARMOR | bonus bVit,1; bonus bMaxHP,100; | PICKY_:1, META_PICKY_:1, C4_PICKY_:1 |
| 4012 | Thief Bug Egg Card | EQP_SHIELD | bonus bMaxHP,400; | THIEF_BUG_EGG:1 |
| 4013 | Andre Egg Card | EQP_SHIELD | bonus bMaxHPrate,5; | ANT_EGG:1, META_ANT_EGG:1 |
| 4014 | Roda Frog Card | EQP_ARMOR | bonus bMaxHP,400; bonus bMaxSP,50; | RODA_FROG:1, C3_RODA_FROG:1, C4_RODA_FROG:1 |
| 4015 | Condor Card | EQP_GARMENT | bonus bFlee,10; | CONDOR:1 |
| 4016 | Thief Bug Card | EQP_ARMOR | bonus bAgi,1; | THIEF_BUG:1, C2_THIEF_BUG:1, C3_THIEF_BUG:1 +1 |
| 4017 | Savage Babe Card | EQP_WEAPON | bonus2 bAddEff,Eff_Stun,500; | SAVAGE_BABE:1, C1_SAVAGE_BABE:1 |
| 4018 | Andre Larva Card | EQP_WEAPON | bonus bInt,1; bonus bMaxSP,10; |  |
| 4019 | Hornet Card | EQP_WEAPON | bonus bStr,1; bonus bBaseAtk,3; | HORNET:1, C5_HORNET:1 |
| 4020 | Farmiliar Card | EQP_WEAPON | bonus2 bAddEff,Eff_Blind,500; bonus bBaseAtk,5; | FARMILIAR:1, C5_FARMILIAR:1 |
| 4021 | Rocker Card | EQP_ARMOR | bonus bDex,1; bonus bBaseAtk,5; | ROCKER:1, C5_ROCKER:1 |
| 4022 | Spore Card | EQP_ACC | bonus bVit,2; | SPORE:1, C1_SPORE:1 |
| 4023 | Desert Wolf Babe Card | EQP_ARMOR | bonus bInt,1; | DESERT_WOLF_B:1, C3_DESERT_WOLF_B:1, C4_DESERT_WOLF_B:1 |
| 4024 | Plankton Card | EQP_WEAPON | bonus2 bAddEff,Eff_Sleep,500; bonus bBaseAtk,5; | PLANKTON:1, E_PLANKTON:1, C4_PLANKTON:1 |
| 4025 | Skeleton Card | EQP_WEAPON | bonus bBaseAtk,10; bonus2 bAddEff,Eff_Stun,200; | SKELETON:1 |
| 4026 | Thief Bug Female Card | EQP_WEAPON | bonus bAgi,1; bonus bFlee,1; | THIEF_BUG_FEMALE:1, THIEF_BUG_:1 |
| 4027 | Kukre Card | EQP_ACC | bonus bAgi,2; | KUKRE:1 |
| 4028 | Tarou Card | EQP_ACC | bonus bStr,2; | TAROU:1, C3_TAROU:1 |
| 4029 | Wolf Card | EQP_WEAPON | bonus bBaseAtk,15; bonus bCritical,1; | WOLF:1, C1_WOLF:1 |
| 4030 | Mandragora Card | EQP_WEAPON | bonus2 bAddEle,Ele_Wind,20; | MANDRAGORA:1, E_MANDRAGORA:1, C2_MANDRAGORA:1 |
| 4031 | Pecopeco Card | EQP_ARMOR | bonus bMaxHPrate,10; | PECOPECO:1, CONCEIVE_PECOPECO:1, C5_PECOPECO:1 |
| 4032 | Ambernite Card | EQP_SHIELD | bonus bDef,2; | AMBERNITE:1, C5_AMBERNITE:1 |
| 4033 | Poporing Card | EQP_ACC | skill TF_DETOXIFY,1; | POPORING:1, E_POPORING:1, C3_POPORING:1 +2 |
| 4034 | Worm Tail Card | EQP_ACC | bonus bDex,2; | WORM_TAIL:1, C3_WORM_TAIL:1 |
| 4035 | Hydra Card | EQP_WEAPON | bonus2 bAddRace,RC_DemiPlayer,20; | HYDRA:1 |
| 4036 | Muka Card | EQP_ACC | bonus bHPrecovRate,10; | MUKA:1, E_MUKA:1, C3_MUKA:1 |
| 4037 | Snake Card | EQP_WEAPON | bonus2 bAddEff,Eff_Poison,500; bonus bBaseAtk,5; | SNAKE:1, C3_SNAKE:1 |
| 4038 | Zombie Card | EQP_SHOES | bonus bHPrecovRate,20; | ZOMBIE:1, E_ZOMBIE:1, C4_ZOMBIE:1 |
| 4039 | Stainer Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Silence,2000; bonus bDef,1; | STAINER:1, C5_STAINER:1 |
| 4040 | Creamy Card | EQP_ACC | skill AL_TELEPORT,1; | CREAMY:1, META_CREAMY:1, C1_CREAMY:1 |
| 4041 | Coco Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Sleep,2000; bonus bDef,1; | COCO:1, E_COCO:1, C1_COCO:1 +1 |
| 4042 | Steel Chonchon Card | EQP_ARMOR | bonus2 bSubEle,Ele_Wind,10; bonus bDef,2; | STEEL_CHONCHON:1, C2_STEEL_CHONCHON:1 |
| 4043 | Andre Card | EQP_WEAPON | bonus bBaseAtk,20; | ANDRE:1, DENIRO:1, PIERE:1 +5 |
| 4044 | Smokie Card | EQP_ACC | skill TF_HIDING,1; | SMOKIE:1, C4_SMOKIE:1 |
| 4045 | Horn Card | EQP_SHIELD | bonus bLongAtkDef,35; | HORN:1, FILAMENTOUS:1, C2_HORN:1 |
| 4046 | Martin Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Blind,2000; bonus bDef,1; | MARTIN:1, E_MARTIN:1, C2_MARTIN:1 |
| 4047 | Ghostring Card | EQP_ARMOR | bonus bDefEle,Ele_Ghost; bonus bHPrecovRate,-25; | GHOSTRING:1 |
| 4048 | Poison Spore Card | EQP_ACC | skill TF_POISON,3; | POISON_SPORE:1, L_POISON_SPORE:1, E_POISONSPORE:1 +1 |
| 4049 | Vadon Card | EQP_WEAPON | bonus2 bAddEle,Ele_Fire,20; | VADON:1, E_VADON:1, E_VADON_X:1 +1 |
| 4050 | Thief Bug Male Card | EQP_SHOES | bonus bAgi,2; | THIEF_BUG_MALE:1, THIEF_BUG__:1 |
| 4051 | Yoyo Card | EQP_ACC | bonus bFlee2,5; bonus bAgi,1; | YOYO:1, L_CHOCO:1, PROVOKE_YOYO:1 +2 |
| 4052 | Elder Wilow Card | EQP_HEAD_LOW | bonus bInt,2; | ELDER_WILOW:1, C1_ELDER_WILOW:1, C2_ELDER_WILOW:1 |
| 4053 | Vitata Card | EQP_ACC | skill AL_HEAL,1; bonus bUseSPrate,25; | VITATA:1 |
| 4054 | Angeling Card | EQP_ARMOR | bonus bDefEle,Ele_Holy; | ANGELING:1 |
| 4055 | Marina Card | EQP_WEAPON | bonus2 bAddEff,Eff_Freeze,500; bonus bBaseAtk,5; | MARINA:1, E_MARINA:1 |
| 4056 | Dustiness Card | EQP_GARMENT | bonus2 bSubEle,Ele_Wind,30; bonus bFlee,5; | DUSTINESS:1, C4_DUSTINESS:1 |
| 4057 | Metaller Card | EQP_WEAPON | bonus2 bAddEff,Eff_Silence,500; bonus bBaseAtk,5; | METALLER:1, C1_METALLER:1 |
| 4058 | Thara Frog Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_DemiPlayer,30; | THARA_FROG:1, C5_THARA_FROG:1 |
| 4059 | Soldier Andre Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Plant,30; | SOLDIER_ANDRE:1, SOLDIER_DENIRO:1, SOLDIER_PIERE:1 |
| 4060 | Goblin Card | EQP_WEAPON | bonus2 bAddRace,RC_Brute,20; | GOBLIN_1:1, GOBLIN_2:1, GOBLIN_3:1 +6 |
| 4061 | Cornutus Card | EQP_ARMOR | bonus bUnbreakableArmor,0; bonus bDef,1; | CORNUTUS:1, C2_CORNUTUS:1 |
| 4062 | Anacondaq Card | EQP_WEAPON | bonus2 bAddEle,Ele_Poison,20; | ANACONDAQ:1, C4_ANACONDAQ:1 |
| 4063 | Caramel Card | EQP_WEAPON | bonus2 bAddRace,RC_Insect,20; | CARAMEL:1, C1_CARAMEL:1 |
| 4064 | Zerom Card | EQP_ACC | bonus bDex,3; | ZEROM:1 |
| 4065 | Kaho Card | EQP_WEAPON | bonus2 bAddEle,Ele_Earth,20; | KAHO:1 |
| 4066 | Orc Warrior Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Brute,30; | ORK_WARRIOR:1, L_HIGH_ORC:1, L_ORC:1 +3 |
| 4067 | Megalodon Card | EQP_SHIELD | bonus2 bResEff,Eff_Freeze,2000; bonus bDef,1; | MEGALODON:1 |
| 4068 | Scorpion Card | EQP_WEAPON | bonus2 bAddRace,RC_Plant,20; | SCORPION:1, C5_SCORPION:1 |
| 4069 | Drainliar Card | EQP_WEAPON | bonus2 bAddEle,Ele_Water,20; | DRAINLIAR:1, C5_DRAINLIAR:1 |
| 4070 | Eggyra Card | EQP_SHOES | bonus bSPrecovRate,15; | EGGYRA:1 |
| 4071 | Orc Zombie Card | EQP_GARMENT | bonus2 bSubEle,Ele_Undead,30; bonus bFlee,5; | ORC_ZOMBIE:1, C5_ORC_ZOMBIE:1 |
| 4072 | Golem Card | EQP_WEAPON | bonus bUnbreakableWeapon,0; bonus bBaseAtk,5; | GOLEM:1, C4_GOLEM:1 |
| 4073 | Pirate Skel Card | EQP_ACC | skill MC_DISCOUNT,5; | PIRATE_SKEL:1 |
| 4074 | BigFoot Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Insect,30; | BIGFOOT:1, E_EDDGA:1, C3_BIGFOOT:1 |
| 4075 | Argos Card | EQP_SHIELD | bonus2 bResEff,Eff_Stone,2000; bonus bDef,1; | ARGOS:1, C4_ARGOS:1 |
| 4076 | Magnolia Card | EQP_WEAPON | bonus2 bAddEff,Eff_Curse,500; bonus bBaseAtk,5; | MAGNOLIA:1, E_MAGNOLIA:1, C5_MAGNOLIA:1 |
| 4077 | Phen Card | EQP_ACC | bonus bNoCastCancel,0; bonus bVariableCastrate,25; | PHEN:1, L_PHEN:1, C5_PHEN:1 |
| 4078 | Savage Card | EQP_ARMOR | bonus bVit,3; | SAVAGE:1, M_SAVAGE:1, C2_SAVAGE:1 +1 |
| 4079 | Mantis Card | EQP_ACC | bonus bStr,3; | MANTIS:1, C1_MANTIS:1 |
| 4080 | Flora Card | EQP_WEAPON | bonus2 bAddRace,RC_Fish,20; | FLORA:1 |
| 4081 | Hode Card | EQP_GARMENT | bonus2 bSubEle,Ele_Earth,30; bonus bFlee,5; | HODE:1, C4_HODE:1 |
| 4082 | Desert Wolf Card | EQP_WEAPON | bonus2 bAddSize,Size_Small,15; bonus bBaseAtk,5; | DESERT_WOLF:1, M_DESERT_WOLF:1 |
| 4083 | Rafflesia Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Fish,30; | RAFFLESIA:1, C1_RAFFLESIA:1 |
| 4084 | Marine Sphere Card | EQP_ACC | skill SM_MAGNUM,3; | MARINE_SPHERE:1 |
| 4085 | Orc Skeleton Card | EQP_WEAPON | bonus2 bAddEle,Ele_Holy,20; | ORC_SKELETON:1, C1_ORC_SKELETON:1 |
| 4086 | Soldier Skeleton Card | EQP_WEAPON | bonus bCritical,9; | SOLDIER_SKELETON:1, C3_SOLDIER_SKELETON:1, C4_SOLDIER_SKELETON:1 |
| 4087 | Giearth Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Confusion,10000; bonus2 bSubEle,Ele_Earth,15; | GIEARTH:1 |
| 4088 | Frilldora Card | EQP_GARMENT | skill AS_CLOAKING,1; | FRILLDORA:1 |
| 4089 | Sword Fish Card | EQP_ARMOR | bonus bDefEle,Ele_Water; bonus bDef,1; | SWORD_FISH:1 |
| 4090 | Munak Card | EQP_SHIELD | bonus2 bResEff,Eff_Stone,1500; bonus2 bSubEle,Ele_Earth,5; bonus bDef,1; | MUNAK:1 |
| 4091 | Kobold Card | EQP_ACC | bonus bStr,1; bonus bCritical,4; | KOBOLD_1:1, KOBOLD_2:1, KOBOLD_3:1 +4 |
| 4092 | Skel Worker Card | EQP_WEAPON | bonus2 bAddSize,Size_Medium,15; bonus bBaseAtk,5; | SKEL_WORKER:1, C5_SKEL_WORKER:1 |
| 4093 | Obeaune Card | EQP_ACC | skill AL_CURE,1; | OBEAUNE:1, E_OBEAUNE:1 |
| 4094 | Archer Skeleton Card | EQP_WEAPON | bonus bLongAtkRate,10; | ARCHER_SKELETON:1 |
| 4095 | Marse Card | EQP_GARMENT | bonus2 bSubEle,Ele_Water,30; bonus bFlee,5; | MARSE:1 |
| 4096 | Zenorc Card | EQP_WEAPON | bonus2 bAddEff,Eff_Poison,400; bonus bBaseAtk,10; | ZENORC:1 |
| 4097 | Matyr Card | EQP_SHOES | bonus bMaxHPrate,10; bonus bAgi,1; | MATYR:1 |
| 4098 | Dokebi Card | EQP_ARMOR | bonus bDefEle,Ele_Wind; bonus bDef,1; | DOKEBI:1, C4_DOKEBI:1 |
| 4099 | Pasana Card | EQP_ARMOR | bonus bDefEle,Ele_Fire; bonus bDef,1; | PASANA:1, C1_PASANA:1 |
| 4100 | Sohee Card | EQP_SHOES | bonus bMaxSPrate,15; bonus bSPrecovRate,3; | SOHEE:1, C1_SOHEE:1 |
| 4101 | Sand Man Card | EQP_ARMOR | bonus bDefEle,Ele_Earth; bonus bDef,1; | SAND_MAN:1, C4_SAND_MAN:1 |
| 4102 | Whisper Card | EQP_GARMENT | bonus bFlee,20; bonus2 bSubEle,Ele_Ghost,-50; | WHISPER:1, C2_WHISPER:1 |
| 4103 | Horong Card | EQP_ACC | skill MG_SIGHT,1; | HORONG:1 |
| 4104 | Requiem Card | EQP_WEAPON | bonus2 bAddEff,Eff_Confusion,500; | REQUIEM:1, C2_REQUIEM:1 |
| 4105 | Marc Card | EQP_ARMOR | bonus2 bSubEle,Ele_Water,5; bonus2 bResEff,Eff_Freeze,10000; | MARC:1, E_MARC:1 |
| 4106 | Mummy Card | EQP_WEAPON | bonus bHit,20; | MUMMY:1, N_MUMMY:1, C1_MUMMY:1 +1 |
| 4107 | Verit Card | EQP_SHOES | bonus bMaxHPrate,8; bonus bMaxSPrate,8; | VERIT:1, N_VERIT:1 |
| 4108 | Myst Card | EQP_GARMENT | bonus2 bSubEle,Ele_Poison,30; bonus bFlee,5; | MYST:1 |
| 4109 | Jakk Card | EQP_GARMENT | bonus2 bSubEle,Ele_Fire,30; bonus bFlee,5; | JAKK:1, JAKK_XMAS:1 |
| 4110 | Ghoul Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Poison,2000; bonus bDef,1; | GHOUL:1, E_GHOUL:1, C4_GHOUL:1 +1 |
| 4111 | Strouf Card | EQP_WEAPON | bonus2 bAddRace,RC_Demon,20; | STROUF:1, E_STROUF:1 |
| 4112 | Marduk Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Silence,10000; | MARDUK:1, C5_MARDUK:1 |
| 4113 | Marionette Card | EQP_GARMENT | bonus2 bSubEle,Ele_Ghost,30; bonus bFlee,5; | MARIONETTE:1, C3_MARIONETTE:1 |
| 4114 | Argiope Card | EQP_ARMOR | bonus bDefEle,Ele_Poison; bonus bDef,1; | ARGIOPE:1, C5_ARGIOPE:1, C1_ARGIOPE:1 |
| 4115 | Hunter Fly Card | EQP_WEAPON | bonus2 bHPDrainRate,30,15; | HUNTER_FLY:1, C4_HUNTER_FLY:1 |
| 4116 | Isis Card | EQP_GARMENT | bonus2 bSubEle,Ele_Dark,30; bonus bFlee,5; | ISIS:1, C2_ISIS:1 |
| 4117 | Side Winder Card | EQP_WEAPON | skill TF_DOUBLE,1; bonus bDoubleRate,5; | SIDE_WINDER:1, C2_SIDE_WINDER:1, C3_SIDE_WINDER:1 |
| 4118 | Petit Card | EQP_WEAPON | bonus2 bAddRace,RC_Dragon,20; | PETIT:1, C1_PETIT:1, C2_PETIT:1 |
| 4119 | Bathory Card | EQP_ARMOR | bonus bDefEle,Ele_Dark; | BATHORY:1, C4_BATHORY:1 |
| 4120 | Petit  Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Dragon,30; | PETIT_:1 |
| 4121 | Phreeoni Card | EQP_WEAPON | bonus bHit,100; | PHREEONI:1 |
| 4122 | Deviruchi Card | EQP_HEAD_LOW | bonus bStr,1; bonus2 bResEff,Eff_Blind,10000; | DEVIRUCHI:1, C2_DEVIRUCHI:1 |
| 4123 | Eddga Card | EQP_SHOES | bonus bMaxHPrate,-25; | EDDGA:1 |
| 4124 | Medusa Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Demon,15; bonus2 bResEff,Eff_Stone,10000; | MEDUSA:1, C1_MEDUSA:1 |
| 4125 | Deviace Card | EQP_WEAPON | bonus2 bAddRace,RC_DemiPlayer,7; bonus2 bAddRace,RC_Brute,7; bonus2 bAddRace,RC_Plant,7; bonus2 bAddRace,RC_Insect,7; | DEVIACE:1 |
| 4126 | Minorous Card | EQP_WEAPON | bonus2 bAddSize,Size_Large,15; bonus bBaseAtk,5; | MINOROUS:1, N_MINOROUS:1, C4_MINOROUS:1 +2 |
| 4127 | Nightmare Card | EQP_HEAD_LOW | bonus2 bResEff,Eff_Sleep,10000; bonus bAgi,1; | NIGHTMARE:1, E_DOPPELGANGER:1 |
| 4128 | Golden Bug Card | EQP_SHIELD | bonus bNoMagicDamage,100; bonus bUseSPrate,100; | GOLDEN_BUG:1 |
| 4129 | Baphomet  Card | EQP_GARMENT | bonus bAgi,3; bonus bCritical,1; | BAPHOMET_:1 |
| 4130 | Scorpion King Card | EQP_WEAPON | bonus2 bAddEle,Ele_Undead,20; | SCORPION_KING:1 |
| 4131 | Moonlight Flower Card | EQP_SHOES | bonus bSpeedRate,25; | MOONLIGHT:1 |
| 4132 | Mistress Card | EQP_HEAD_LOW | bonus bNoGemStone,0; bonus bUseSPrate,25; | MISTRESS:1, E_MISTRESS:1 |
| 4133 | Daydric Card | EQP_GARMENT | bonus2 bSubEle,Ele_Neutral,20; | RAYDRIC:1, C1_RAYDRIC:1, C2_RAYDRIC:1 |
| 4134 | Dracula Card | EQP_WEAPON | bonus2 bSPDrainRate,100,5; | DRACULA:1, E_DRACULA:1 |
| 4135 | Orc Load Card | EQP_ARMOR | bonus bShortWeaponDamageReturn,30; | ORC_LORD:1 |
| 4136 | Khalitzburg Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Demon,30; | KHALITZBURG:1 |
| 4137 | Drake Card | EQP_WEAPON | bonus bNoSizeFix,0; | DRAKE:1 |
| 4138 | Anubis Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Angel,30; | ANUBIS:1 |
| 4139 | Joker Card | EQP_ACC | skill TF_STEAL,1; | JOKER:1 |
| 4140 | Knight Of Abyss Card | EQP_WEAPON | bonus2 bAddRace,RC_Boss,25; | KNIGHT_OF_ABYSS:1 |
| 4141 | Evil Druid Card | EQP_ARMOR | bonus bDefEle,Ele_Undead; bonus bInt,1; bonus bDef,1; | EVIL_DRUID:1, C5_EVIL_DRUID:1 |
| 4142 | Doppelganger Card | EQP_WEAPON | bonus bAspdRate,10; | DOPPELGANGER:1 |
| 4143 | Orc Hero Card | EQP_HEAD_LOW | bonus bVit,3; bonus2 bResEff,Eff_Stun,10000; | ORK_HERO:1, E_ORK_HERO2:1 |
| 4144 | Osiris Card | EQP_ACC | bonus bRestartFullRecover,0; | OSIRIS:1, E_OSIRIS2:1 |
| 4145 | Berzebub Card | EQP_ACC | bonus bVariableCastrate,-30; | BEELZEBUB_:1 |
| 4146 | Maya Card | EQP_SHIELD | bonus bMagicDamageReturn,50; | MAYA:1 |
| 4147 | Baphomet Card | EQP_WEAPON | bonus bHit,-10; bonus bSplashRange,1; | BAPHOMET:1, BAPHOMET_I:1, E_BAPHOMET2:1 |
| 4148 | Pharaoh Card | EQP_HEAD_LOW | bonus bUseSPrate,-30; | PHARAOH:1 |
| 4149 | Gargoyle Card | EQP_ACC | bonus3 bAddMonsterDropItem,12028,RC_Insect,100; | GARGOYLE:1, C3_GARGOYLE:1, C4_GARGOYLE:1 |
| 4150 | Goat Card | EQP_ARMOR |  | GOAT:1, C2_GOAT:1, C3_GOAT:1 |
| 4151 | Gajomart Card | EQP_SHOES | bonus2 bSubRace,RC_Plant,-20; bonus2 bExpAddRace,RC_Plant,10; | GAJOMART:1 |
| 4152 | Galapago Card | EQP_ACC | bonus2 bAddItemHealRate,Apple_Juice,50; bonus2 bAddItemHealRate,Banana_Juice,50; bonus2 bAddItemHealRate,Carrot_Juice,50 | GALAPAGO:1 |
| 4153 | Crab Card | EQP_WEAPON | bonus bBaseAtk,5; bonus2 bAddDamageClass,1266,30; | CRAB:1 |
| 4154 | Rice Cake Boy Card | EQP_ACC | bonus2 bAddItemHealRate,Candy,50; bonus2 bAddItemHealRate,Candy_Striper,50; bonus3 bAddMonsterDropItem,529,RC_DemiPlayer | RICE_CAKE_BOY:1, C3_RICE_CAKE_BOY:1 |
| 4155 | Goblin Leader Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Goblin,30; | GOBLIN_LEADER:1 |
| 4156 | Steam Goblin Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Formless,7; | STEAM_GOBLIN:1 |
| 4157 | Goblin Archer Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Undead,7; | GOBLIN_ARCHER:1 |
| 4158 | Flying Deleter Card | EQP_ARMOR | bonus bHPrecovRate,-100; bonus bHPGainValue,100; | DELETER:1 |
| 4159 | Nine Tail Card | EQP_GARMENT | bonus bAgi,2; if(getrefine()>8) bonus bFlee,20; | NINE_TAIL:1 |
| 4160 | Antique Firelock Card | EQP_SHOES |  | ANTIQUE_FIRELOCK:1 |
| 4161 | Grand Peco Card | EQP_HEAD_LOW | bonus3 bAutoSpellWhenHit,PR_GLORIA,1,50; | GRAND_PECO:1, C2_GRAND_PECO:1, C3_GRAND_PECO:1 |
| 4162 | Grizzly Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Blind,300; | GRIZZLY:1 |
| 4163 | Gryphon Card | EQP_WEAPON | bonus bFlee,2; bonus bCritical,7; if(BaseClass==Job_Swordman) bonus3 bAutoSpell,KN_BOWLINGBASH,5,10; | GRYPHON:1 |
| 4164 | Gullinbursti Card | EQP_SHOES | bonus2 bSubRace,RC_Fish,-20; bonus2 bExpAddRace,RC_Fish,10; | GULLINBURSTI:1 |
| 4165 | Gig Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Insect,5; | GIG:1 |
| 4166 | Nightmare Terror Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Curse,300; | NIGHTMARE_TERROR:1, C4_NIGHTMARE_TERROR:1 |
| 4167 | Neraid Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Brute,5; | NERAID:1 |
| 4168 | Dark Lord Card | EQP_SHOES | bonus3 bAutoSpellWhenHit,WZ_METEOR,5,100; | DARK_LORD:1 |
| 4169 | Dark Illusion Card | EQP_HEAD_LOW | bonus bMaxHPrate,-10; bonus bMaxSPrate,-10; bonus bVariableCastrate,-10; | DARK_ILLUSION:1, E_DARK_LORD:1 |
| 4170 | Dark Frame Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Stone,600; | DARK_FRAME:1 |
| 4171 | Dark Priest Card | EQP_WEAPON | bonus2 bSPVanishRate, 50, 10; if (BaseJob == Job_Sage) bonus bSPDrainValue, 1; | DARK_PRIEST:1, C4_DARK_PRIEST:1 |
| 4172 | The Paper Card | EQP_WEAPON | bonus bCritAtkRate,20; bonus2 bSPDrainValue,-1,0; | THE_PAPER:1 |
| 4173 | Demon Pungus Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Sleep,600; | DEMON_PUNGUS:1 |
| 4174 | Deviling Card | EQP_GARMENT | bonus2 bSubEle,Ele_Neutral,50; bonus2 bSubEle,Ele_Water,-50; bonus2 bSubEle,Ele_Earth,-50; bonus2 bSubEle,Ele_Fire,-50;  | DEVILING:1 |
| 4175 | Poison Toad Card | EQP_ACC | bonus3 bAutoSpell,TF_POISON,1,20; bonus2 bAddSkillBlow,52,5; | POISON_TOAD:1, C1_POISON_TOAD:1 |
| 4176 | Dullahan Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Dragon,7; | DULLAHAN:1 |
| 4177 | Dryad Card | EQP_HEAD_LOW | bonus3 bAddMonsterDropItem,993,RC_Plant,100; bonus2 bSubEle,Ele_Earth,10; | DRYAD:1, C1_DRYAD:1 |
| 4178 | Dragon Tail Card | EQP_GARMENT | bonus bAgi,1; bonus bFlee,10; bonus2 bSkillAtk,AC_DOUBLE,5; bonus2 bSkillAtk,AC_SHOWER,5; | DRAGON_TAIL:1, C1_DRAGON_TAIL:1 |
| 4179 | Dragon Fly Card | EQP_GARMENT | bonus bAgi,1; | DRAGON_FLY:1 |
| 4180 | Driller Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Dragon,5; | DRILLER:1, C4_DRILLER:1 |
| 4181 | Disguise Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Silence,300+600*(readparam(bVit)>=77); | DISGUISE:1, C5_DISGUISE:1 |
| 4182 | Diabolic Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Demon,5; | DIABOLIC:1 |
| 4183 | Vagabond Wolf Card | EQP_GARMENT | bonus bStr,1; | VAGABOND_WOLF:1 |
| 4184 | Lava Golem Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Golem,30; | LAVA_GOLEM:1 |
| 4185 | Rideword Card | EQP_HEAD_LOW |  | RIDEWORD:1, C1_RIDEWORD:1, C2_RIDEWORD:1 |
| 4186 | Raggler Card | EQP_SHOES | bonus bStr,1; bonus bVit,1; | RAGGLER:1, C5_RAGGLER:1 |
| 4187 | Raydric Archer Card | EQP_ACC | bonus3 bAddMonsterDropItem,12030,RC_Demon,100; | RAYDRIC_ARCHER:1 |
| 4188 | Leib Olmai Card | EQP_HEAD_LOW | bonus2 bSubEle,Ele_Fire,10; bonus3 bAddMonsterDropItem,990,RC_Brute,100; | LEIB_OLMAI:1, C2_LEIB_OLMAI:1 |
| 4189 | Wraith Dead Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Curse,600; | WRAITH_DEAD:1 |
| 4190 | Wraith Card | EQP_ACC | bonus3 bAddMonsterDropItem,12027,RC_Undead,100; | WRAITH:1 |
| 4191 | Loli Ruri Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,AL_HEAL,3,50; | LOLI_RURI:1, C1_LOLI_RURI:1 |
| 4192 | Rotar Zairo Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Fish,7; | ROTAR_ZAIRO:1 |
| 4193 | Lude Card | EQP_ACC | if(BaseJob==Job_Novice--BaseJob==Job_SuperNovice) bonus3 bAutoSpellWhenHit,SM_ENDURE,1,200; | LUDE:1, C4_LUDE:1 |
| 4194 | Rybio Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Stun,300+600*(readparam(bDex)>=77); | RYBIO:1 |
| 4195 | Leaf Cat Card | EQP_HEAD_LOW | bonus2 bSubEle,Ele_Water,10; bonus3 bAddMonsterDropItem,991,RC_Fish,100; | LEAF_CAT:1, C3_LEAF_CAT:1 |
| 4196 | Marin Card | EQP_ACC | bonus2 bAddMonsterDropItem,909,2000; bonus2 bAddMonsterDropItem,7126,10; | MARIN:1, E_MARIN:1, C4_MARIN:1 |
| 4197 | Mastering Card | EQP_GARMENT | bonus bLuk,1; | MASTERING:1 |
| 4198 | Maya Puple Card | EQP_HEAD_LOW | bonus bIntravision,0; | MAYA_PUPLE:1 |
| 4199 | Merman Card | EQP_SHOES | bonus bHPrecovRate,10; bonus bSPrecovRate,10; | MERMAN:1, C4_MERMAN:1 |
| 4200 | Megalith Card | EQP_SHOES | if(getrefine()<6) bonus bMdef,7; | MEGALITH:1 |
| 4201 | Majoruros Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Stun,600; | MAJORUROS:1, C4_MAJORUROS:1 |
| 4202 | Civil Servant Card | EQP_WEAPON | bonus2 bAddEle,Ele_Ghost,20; | CIVIL_SERVANT:1 |
| 4203 | Mutant Dragon Card | EQP_WEAPON | bonus bBaseAtk,15; bonus3 bAutoSpell,MG_FIREBALL,3+2*(getskilllv(MG_FIREBALL)==10),50; | MUTANT_DRAGON:1 |
| 4204 | Mini Demon Card | EQP_SHOES | bonus2 bSubRace,RC_Brute,-20; bonus2 bExpAddRace,RC_Brute,10; | MINI_DEMON:1 |
| 4205 | Mimic Card | EQP_ACC | bonus2 bAddMonsterDropItem,603,10; | MIMIC:1, N_MIMIC:1, C4_MIMIC:1 +1 |
| 4206 | Mystcase Card | EQP_HEAD_LOW | bonus2 bAddMonsterDropItem,644,30; | MYSTCASE:1, C4_MYSTCASE:1 |
| 4207 | Mysteltainn Card | EQP_SHIELD | bonus2 bSubSize,Size_Small,25; bonus bDef,1; | MYSTELTAINN:1 |
| 4208 | Miyabi Ningyo Card | EQP_SHOES | bonus bMaxSPrate,10; bonus2 bSkillAtk,MG_FROSTDIVER,5; | MIYABI_NINGYO:1, C3_MIYABI_NINGYO:1 |
| 4209 | Violy Card | EQP_ACC | if (getskilllv(BA_FROSTJOKE) == 5) bonus3(bAutoSpell, BA_FROSTJOKE, 5, 20); else bonus3(bAutoSpell, BA_FROSTJOKE, 1, 20) | VIOLY:1, C5_VIOLY:1, C1_VIOLY:1 +1 |
| 4210 | Wander Man Card | EQP_GARMENT | if(BaseClass==Job_Thief) bonus bFlee,20; | WANDER_MAN:1, E_LORD_OF_DEATH2:1, C4_WANDER_MAN:1 |
| 4211 | Vocal Card | EQP_GARMENT | bonus bMdef,3; | VOCAL:1 |
| 4212 | Bon Gun Card | EQP_ACC | bonus3 bAutoSpell,SM_BASH,1,20; bonus2 bAddSkillBlow,SM_BASH,5; bonus2 bAddDefClass,1026,-100; | BON_GUN:1 |
| 4213 | Brilight Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Silence,600; | BRILIGHT:1 |
| 4214 | Bloody Murderer Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Insect,7; | BLOODY_MURDERER:1 |
| 4215 | Blazzer Card | EQP_ACC | bonus bAddMonsterDropChainItem,ITMCHAIN_FOOD; | BLAZZER:1 |
| 4216 | Sasquatch Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Freeze,600; | SASQUATCH:1 |
| 4217 | Live Peach Tree Card | EQP_SHIELD | bonus3 bAutoSpell,AL_HEAL,1+9*(getskilllv(AL_HEAL)==10),20; | LIVE_PEACH_TREE:1, C2_LIVE_PEACH_TREE:1 |
| 4218 | Succubus Card | EQP_ARMOR | bonus bVit,-3; bonus bHPrecovRate,-20; bonus bMaxHP,1000; | SUCCUBUS:1 |
| 4219 | Sageworm Card | EQP_ACC | bonus2 bAddMonsterDropItem,715,30; bonus2 bAddMonsterDropItem,716,30; bonus2 bAddMonsterDropItem,717,30; | SAGEWORM:1 |
| 4220 | Solider Card | EQP_ARMOR | bonus bDef,2; bonus bMdef,2; | SOLIDER:1, C2_SOLIDER:1 |
| 4221 | Skeleton General Card | EQP_SHOES | bonus2 bSubRace,RC_Insect,-20; bonus2 bExpAddRace,RC_Insect,10; | SKELETON_GENERAL:1, C3_SKELETON_GENERAL:1, C4_SKELETON_GENERAL:1 |
| 4222 | Skel Prisoner Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Sleep,300; | SKEL_PRISONER:1 |
| 4223 | Stalactic Golem Card | EQP_HEAD_LOW | bonus bDef,1; bonus2 bResEff,Eff_Stun,2000; | STALACTIC_GOLEM:1, C4_STALACTIC_GOLEM:1 |
| 4224 | Stem Worm Card | EQP_ACC | bonus3 bAddMonsterDropItem,12032,RC_Brute,100; | STEM_WORM:1, C1_STEM_WORM:1 |
| 4225 | Stone Shooter Card | EQP_WEAPON | bonus bBaseAtk,10; bonus bHit,10; | STONE_SHOOTER:1 |
| 4226 | Sting Card | EQP_SHIELD | bonus bDef,2; if(getrefine()>8) bonus bMdef,5; | STING:1, C5_STING:1 |
| 4227 | Spring Rabbit Card | EQP_ACC | bonus2 bAddItemHealRate,Meat,50; bonus2 bAddItemHealRate,528,50; bonus3 bAddMonsterDropItem,Meat,RC_Brute,200; bonus3 bA | SPRING_RABBIT:1 |
| 4228 | Sleeper Card | EQP_ACC | bonus3 bAddMonsterDropItem,12031,RC_Fish,100; | SLEEPER:1, C5_SLEEPER:1, C1_SLEEPER:1 |
| 4229 | C Tower Manager Card | EQP_HEAD_LOW | bonus bInt,1; bonus bVariableCastrate,-5; | C_TOWER_MANAGER:1 |
| 4230 | Shinobi Card | EQP_ACC | bonus bAgi,1; bonus3 bAutoSpellWhenHit,AS_CLOAKING,5,100; | SHINOBI:1, C4_SHINOBI:1 |
| 4231 | Increase Soil Card | EQP_SHIELD | bonus2 bSubRace2,RC2_Guardian,50; | INCREASE_SOIL:1, C5_INCREASE_SOIL:1 |
| 4232 | Wild Ginseng Card | EQP_ACC | bonus2 bAddItemHealRate,Red_Herb,50; bonus2 bAddItemHealRate,Yellow_Herb,50; bonus2 bAddItemHealRate,White_Herb,50; bonu | WILD_GINSENG:1 |
| 4233 | Baby Leopard Card | EQP_ARMOR | bonus bLuk,3; if(BaseClass==Job_Merchant) bonus bUnbreakableArmor,0; | BABY_LEOPARD:1 |
| 4234 | Anolian Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,AC_CONCENTRATION,1+9*(getskilllv(AC_CONCENTRATION)==10),30; | ANOLIAN:1, C4_ANOLIAN:1 |
| 4235 | Cookie XMAS Card | EQP_SHOES | bonus2 bSubRace,RC_Angel,-20; bonus2 bExpAddRace,RC_Angel,10; | COOKIE_XMAS:1 |
| 4236 | Amon Ra Card | EQP_SHOES | bonus bAllStats,1; bonus3 bAutoSpellWhenHit,PR_KYRIE,10,(30+70*(readparam(bInt)>=99)); | AMON_RA:1, N_AMON_RA:1 |
| 4237 | Owl Duke Card | EQP_ACC | bonus3 bAutoSpell,PR_IMPOSITIO,3,3; | OWL_DUKE:1, C3_OWL_DUKE:1 |
| 4238 | Owl Baron Card | EQP_ACC | bonus3 bAutoSpell,PR_LEXAETERNA,1,30; | OWL_BARON:1 |
| 4239 | Iron Fist Card | EQP_SHOES | bonus2 bSubRace,RC_Formless,-20; bonus2 bExpAddRace,RC_Formless,10; | IRON_FIST:1 |
| 4240 | Arclouse Card | EQP_SHIELD |  | ARCLOUSE:1, N_ARCLOUSE:1, C2_ARCLOUSE:1 +1 |
| 4241 | Archangeling Card | EQP_HEAD_LOW |  | ARCHANGELING:1 |
| 4242 | Apocalips Card | EQP_ARMOR | bonus bVit,2; if(getrefine()>8) bonus bMaxHP,800; | APOCALIPS:1, C4_APOCALIPS:1 |
| 4243 | Antonio Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,AL_TELEPORT,1,500; |  |
| 4244 | Alarm Card | EQP_SHOES | bonus3 bAutoSpellWhenHit,MG_SIGHT,1,200; bonus bMaxHP,300; bonus bVit,1; | ALARM:1, C5_ALARM:1 |
| 4245 | Am Mut Card | EQP_SHOES | bonus2 bSubRace,RC_DemiPlayer,-20; bonus2 bExpAddRace,RC_DemiPlayer,10; | AM_MUT:1 |
| 4246 | Assulter Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_DemiPlayer,7; | ASSULTER:1, E_TURTLE_GENERAL:1, C3_ASSULTER:1 |
| 4247 | Aster Card | EQP_WEAPON | bonus bBaseAtk,5; bonus2 bAddDamageClass,1074,30; | ASTER:1 |
| 4248 | Ancient Mummy Card | EQP_SHIELD | bonus3 bAutoSpellWhenHit,AL_CRUCIS,5,30; | ANCIENT_MUMMY:1, N_ANCIENT_MUMMY:1 |
| 4249 | Ancient Worm Card | EQP_SHOES | bonus2 bSubRace,RC_Demon,-20; bonus2 bExpAddRace,RC_Demon,10; | ANCIENT_WORM:1 |
| 4250 | Executioner Card | EQP_SHIELD | bonus2 bSubSize,Size_Large,25; bonus bDef,1; | EXECUTIONER:1 |
| 4251 | Elder Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Guardian,40; | ELDER:1 |
| 4252 | Alligator Card | EQP_ACC | bonus bLongAtkDef,5; | ALLIGATOR:1, C2_ALLIGATOR:1 |
| 4253 | Alice Card | EQP_SHIELD | bonus2 bSubRace,RC_Boss,40; bonus2 bSubRace,RC_NonBoss,-40; | ALICE:1 |
| 4254 | Tirfing Card | EQP_SHIELD | bonus2 bSubSize,Size_Medium,25; bonus bDef,1; | TIRFING:1 |
| 4255 | Orc Lady Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Orc,30; | ORC_LADY:1, C2_ORC_LADY:1 |
| 4256 | Orc Archer Card | EQP_ACC | bonus3 bAddMonsterDropItem,12034,RC_DemiPlayer,100; | ORC_ARCHER:1 |
| 4257 | Wild Rose Card | EQP_SHOES | bonus bAgi,1; if(BaseClass==Job_Thief) bonus bFlee2,5; | WILD_ROSE:1, C4_WILD_ROSE:1 |
| 4258 | Wicked Nymph Card | EQP_HEAD_LOW | bonus bInt,1; bonus bMaxSP,50; | WICKED_NYMPH:1, C5_WICKED_NYMPH:1 |
| 4259 | Wooden Golem Card | EQP_ARMOR | bonus bDef,1; bonus bHPrecovRate,30; | WOODEN_GOLEM:1 |
| 4260 | Wootan Shooter Card | EQP_HEAD_LOW | bonus bDef,1; bonus2 bResEff,Eff_Confusion,2000; | WOOTAN_SHOOTER:1 |
| 4261 | Wootan Fighter Card | EQP_HEAD_LOW | bonus bDef,1; bonus2 bResEff,Eff_Bleeding,2000; | WOOTAN_FIGHTER:1, C4_WOOTAN_FIGHTER:1 |
| 4262 | Evil Cloud Hermit Card | EQP_ACC | bonus3 bAddMonsterDropItem,12029,RC_Plant,100; | EVIL_CLOUD_HERMIT:1 |
| 4263 | Incant Samurai Card | EQP_WEAPON | bonus bIgnoreDefRace,RC_NonBoss; bonus bHPrecovRate,-100; bonus2 bHPLossRate,666,10000; | INCANTATION_SAMURAI:1 |
| 4264 | Wind Ghost Card | EQP_ACC | bonus3 bAutoSpell,WZ_JUPITEL,3+7*(getskilllv(WZ_JUPITEL)==10),20; | WIND_GHOST:1, C2_WIND_GHOST:1 |
| 4265 | Li Me Mang Ryang Card | EQP_ACC | bonus3 bAddMonsterDropItem,12033,RC_Angel,100; | LI_ME_MANG_RYANG:1, C5_LI_ME_MANG_RYANG:1 |
| 4266 | Eclipse Card | EQP_GARMENT | bonus bVit,1; | ECLIPSE:1 |
| 4267 | Explosion Card | EQP_SHOES | bonus2 bSubRace,RC_Dragon,-20; bonus2 bExpAddRace,RC_Dragon,10; | EXPLOSION:1, C4_EXPLOSION:1 |
| 4268 | Injustice Card | EQP_WEAPON | bonus3 bAutoSpell,AS_SONICBLOW,1,50; | INJUSTICE:1, C4_INJUSTICE:1 |
| 4269 | Incubus Card | EQP_HEAD_LOW | bonus bInt,-3; bonus bSPrecovRate,-20; bonus bMaxSP,150; | INCUBUS:1 |
| 4270 | Giant Spider Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Poison,600; | GIANT_SPIDER:1 |
| 4271 | Giant Honet Card | EQP_HEAD_LOW | bonus2 bSubEle,Ele_Wind,10; bonus3 bAddMonsterDropItem,992,RC_Insect,100; | GIANT_HONET:1, C3_GIANT_HONET:1 |
| 4272 | Dancing Dragon Card | EQP_ACC | bonus bAgi,1; bonus bCritical,3; | DANCING_DRAGON:1, C5_DANCING_DRAGON:1 |
| 4273 | Shellfish Card | EQP_WEAPON | bonus bBaseAtk,5; bonus2 bAddDamageClass,1073,30; | SHELLFISH:1, C1_SHELLFISH:1 |
| 4274 | Zombie Master Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Undead,5; | ZOMBIE_MASTER:1, C3_ZOMBIE_MASTER:1 |
| 4275 | Zombie Prisoner Card | EQP_SHOES | bonus2 bSubRace,RC_Undead,-20; bonus2 bExpAddRace,RC_Undead,10; | ZOMBIE_PRISONER:1, C2_ZOMBIE_PRISONER:1 |
| 4276 | Lord Of Death Card | EQP_WEAPON | bonus3 bAddEff,Eff_Stun,500,ATF_SHORT; bonus3 bAddEff,Eff_Curse,500,ATF_SHORT; bonus3 bAddEff,Eff_Silence,500,ATF_SHORT; | LORD_OF_DEATH:1 |
| 4277 | Zherlthsh Card | EQP_SHIELD | bonus bLuk,2; bonus2 bSkillAtk,BA_MUSICALSTRIKE,10; bonus2 bSkillAtk,DC_THROWARROW,10; | ZHERLTHSH:1 |
| 4278 | Gibbet Card | EQP_HEAD_LOW | if(getrefine()<6) bonus bMdef,5; | GIBBET:1 |
| 4279 | Deleter Card | EQP_ARMOR | bonus bSPrecovRate,-100; bonus bSPGainValue,10; | DELETER_:1 |
| 4280 | Geographer Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,AL_BLESSING,2+8*(getskilllv(AL_BLESSING)==10),30; | GEOGRAPHER:1, C1_GEOGRAPHER:1, C2_GEOGRAPHER:1 |
| 4281 | Zipper Bear Card | EQP_WEAPON | bonus bBaseAtk,30; bonus2 bSPDrainValue,-1,0; if(BaseClass==Job_Merchant) bonus bUnbreakableWeapon,0; | ZIPPER_BEAR:1 |
| 4282 | Tengu Card | EQP_ACC | bonus bAddMonsterDropChainItem,ITMCHAIN_HEAL; | TENGU:1 |
| 4283 | Greatest General Card | EQP_ACC | bonus3 bAutoSpell,MO_CALLSPIRITS,5,2+18*(BaseClass==Job_Acolyte); | GREATEST_GENERAL:1, C1_GREATEST_GENERAL:1 |
| 4284 | Chepet Card | EQP_WEAPON | bonus4 bAutoSpell,AL_HEAL,5,50,1; | CHEPET:1 |
| 4285 | Choco Card | EQP_GARMENT | bonus bFlee2,5; bonus bFlee,10; | CHOCO:1, E_CHOCO:1 |
| 4286 | Karakasa Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Confusion,300+600*(readparam(bStr)>=77); | KARAKASA:1, C1_KARAKASA:1 |
| 4287 | Kapha Card | EQP_GARMENT | if(getrefine()<6) bonus bMdef,8; | KAPHA:1 |
| 4288 | Carat Card | EQP_HEAD_LOW | bonus bInt,2; if(getrefine()>8) bonus bMaxSP,150; | CARAT:1, C5_CARAT:1 |
| 4289 | Caterpillar Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Plant,5; | CATERPILLAR:1, C4_CATERPILLAR:1 |
| 4290 | Cat O Nine Tail Card | EQP_SHOES | bonus bMdef,3; bonus bMagicDamageReturn,5; | CAT_O_NINE_TAIL:1 |
| 4291 | Kobold Leader Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Kobold,30; | KOBOLD_LEADER:1 |
| 4292 | Kobold Archer Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Plant,7; | KOBOLD_ARCHER:1, C1_KOBOLD_ARCHER:1 |
| 4293 | Cookie Card | EQP_ACC | bonus bLuk,2; bonus2 bSkillAtk,AL_HOLYLIGHT,10; | COOKIE:1, C3_COOKIE:1 |
| 4294 | Quve Card | EQP_ACC | if(BaseJob==Job_Novice--BaseJob==Job_SuperNovice) bonus3 bAutoSpellWhenHit,AL_INCAGI,1,100; | QUVE:1 |
| 4295 | Kraben Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Blind,600; | KRABEN:1 |
| 4296 | Cramp Card | EQP_HEAD_LOW | bonus2 bGetZenyNum,500,1; | CRAMP:1 |
| 4297 | Cruiser Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Brute,7; | CRUISER:1 |
| 4298 | Cremy Fear Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Confusion,600; | CREMY_FEAR:1 |
| 4299 | Clock Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,CR_AUTOGUARD,3+7*(getskilllv(CR_AUTOGUARD)==10),30; | CLOCK:1, C3_CLOCK:1, C4_CLOCK:1 |
| 4300 | Chimera Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Poison,300+600*(BaseJob==Job_Assassin); | CHIMERA:1 |
| 4301 | Killer Mantis Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Bleeding,600; | KILLER_MANTIS:1 |
| 4302 | Tao Gunka Card | EQP_ARMOR | bonus bMaxHPrate,100; bonus bDefRate,-50; bonus bMdefRate,-50; | TAO_GUNKA:1 |
| 4303 | Whisper Boss Card | EQP_GARMENT |  | WHISPER_BOSS:1 |
| 4304 | Tamruan Card | EQP_SHIELD | bonus bDef,2; bonus2 bSkillAtk,CR_SHIELDCHARGE,10; bonus2 bSkillAtk,CR_SHIELDBOOMERANG,10; | TAMRUAN:1, C4_TAMRUAN:1 |
| 4305 | Turtle General Card | EQP_WEAPON | bonus2 bAddRace, RC_All, 20; bonus3 bAutoSpell,SM_MAGNUM,10,30; | TURTLE_GENERAL:1 |
| 4306 | Toad Card | EQP_GARMENT | bonus bFlee2,1; | TOAD:1 |
| 4307 | Kind Of Beetle Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Fish,5; | KIND_OF_BEETLE:1, C4_KIND_OF_BEETLE:1 |
| 4308 | Tri Joint Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Formless,5; | TRI_JOINT:1 |
| 4309 | Parasite Card | EQP_SHIELD | bonus bDef,1; bonus2 bAddRaceTolerance,RC_Formless,5; | PARASITE:1, C2_PARASITE:1 |
| 4310 | Panzer Goblin Card | EQP_WEAPON | bonus bCritAtkRate,10; bonus2 bCriticalAddRace,RC_Demon,7; | PANZER_GOBLIN:1 |
| 4311 | Permeter Card | EQP_HEAD_LOW | bonus2 bSubEle,Ele_Dark,15; bonus2 bSubEle,Ele_Undead,15; | PERMETER:1 |
| 4312 | Fur Seal Card | EQP_WEAPON |  | FUR_SEAL:1, C1_FUR_SEAL:1 |
| 4313 | Punk Card | EQP_GARMENT | bonus4 bAutoSpellWhenHit,WZ_QUAGMIRE,1+4*(getskilllv(WZ_QUAGMIRE)==5),50,0; | PUNK:1 |
| 4314 | Penomena Card | EQP_SHIELD | bonus2 bSubRace,RC_Formless,30; | PENOMENA:1, C3_PENOMENA:1, C4_PENOMENA:1 |
| 4315 | Pest Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Stone,300+600*(readparam(bInt)>=77); | PEST:1 |
| 4316 | Fake Angel Card | EQP_WEAPON | bonus2 bSPGainRace,RC_Angel,5; | FAKE_ANGEL:1, C1_FAKE_ANGEL:1 |
| 4317 | Mobster Card | EQP_WEAPON | bonus bCritAtkRate,15; if(BaseClass==Job_Thief) bonus bCritical,4; | MOBSTER:1 |
| 4318 | Knight Windstorm Card | EQP_WEAPON | bonus3 bAutoSpell,WZ_STORMGUST,2,20; bonus2 bAddEff,Eff_Freeze,2000; | KNIGHT_OF_WINDSTORM:1 |
| 4319 | Freezer Card | EQP_SHOES | bonus bMaxHP,300; if(getrefine()>=9) bonus2 bSkillAtk,SM_BASH,10; | FREEZER:1, C2_FREEZER:1, C3_FREEZER:1 |
| 4320 | Bloody Knight Card | EQP_WEAPON | bonus3 bAutoSpell,WZ_METEOR,1,20; | BLOODY_KNIGHT:1 |
| 4321 | Hylozoist Card | EQP_ACC | bonus bClassChange,100; | HYLOZOIST:1 |
| 4322 | High Orc Card | EQP_SHIELD | bonus bDef,1; bonus bShortWeaponDamageReturn,5; | HIGH_ORC:1, C2_HIGH_ORC:1 |
| 4323 | Garm Baby Card | EQP_WEAPON | bonus3 bAutoSpell,MG_FROSTDIVER,3,50; | GARM_BABY:1 |
| 4324 | Garm Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Freeze,5000; | GARM:1 |
| 4325 | Harpy Card | EQP_GARMENT | bonus2 bAddRaceTolerance,RC_Formless,15; bonus2 bSkillAtk,MG_NAPALMBEAT,5; | HARPY:1, C3_HARPY:1, C4_HARPY:1 |
| 4326 | See Otter Card | EQP_ACC | bonus2(bAddItemHealRate, Shusi, 50); bonus2(bAddItemHealRate, Fish_Slice, 50); bonus3(bAddMonsterDropItem, Shusi, RC_Fis | SEE_OTTER:1, C3_SEE_OTTER:1 |
| 4327 | Blood Butterfly Card | EQP_ACC | bonus bVariableCastrate,30; bonus bNoCastCancel,0; bonus2 bSkillAtk,MG_FIREWALL,5; | BLOOD_BUTTERFLY:1, C1_BLOOD_BUTTERFLY:1 |
| 4328 | Hyegun Card | EQP_GARMENT | bonus bFlee,15; bonus bCritical,1; | HYEGUN:1, C3_HYEGUN:1 |
| 4329 | Phendark Card | EQP_WEAPON | bonus2 bSPGainRace,RC_DemiPlayer,5; | PHENDARK:1 |
| 4330 | Dark Snake Lord Card | EQP_HEAD_LOW | bonus bInt,3; bonus2 bResEff,Eff_Blind,10000; bonus2 bResEff,Eff_Curse,10000; | DARK_SNAKE_LORD:1, E_DARK_SNAKE_LORD:1 |
| 4331 | Heater Card | EQP_ACC | bonus bCritical,3; if(BaseClass==Job_Swordman) bonus bFlee2,3; | HEATER:1 |
| 4332 | Waste Stove Card | EQP_ARMOR | bonus bBaseAtk,5; bonus bInt,1; | WASTE_STOVE:1 |
| 4333 | Venomous Card | EQP_ARMOR | bonus3 bAddEffWhenHit,Eff_Poison,3000,ATF_TARGET-ATF_SELF; | VENOMOUS:1, C3_VENOMOUS:1 |
| 4334 | Noxious Card | EQP_GARMENT | bonus bLongAtkDef,10; bonus2 bSubEle,Ele_Neutral,10; | NOXIOUS:1, C4_NOXIOUS:1 |
| 4335 | Pitman Card | EQP_WEAPON | bonus2 bSkillAtk,WZ_EARTHSPIKE,5; bonus2 bSkillAtk,WZ_HEAVENDRIVE,5; | PITMAN:1, C5_PITMAN:1 |
| 4336 | Ungoliant Card | EQP_HEAD_LOW | bonus bHPrecovRate,10; bonus2 bResEff,Eff_Bleeding,10000; | UNGOLIANT:1, C3_UNGOLIANT:1 |
| 4337 | Porcellio Card | EQP_ARMOR | bonus bBaseAtk,25; bonus bDef,-5; | PORCELLIO:1, C2_PORCELLIO:1 |
| 4338 | Obsidian Card | EQP_ARMOR | bonus bVit,readparam(bDex)/18; | OBSIDIAN:1 |
| 4339 | Mineral Card | EQP_ARMOR | bonus bBaseAtk,-25; bonus bDef,3; | MINERAL:1, C2_MINERAL:1 |
| 4340 | Teddy Bear Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Undead,30; | TEDDY_BEAR:1, C1_TEDDY_BEAR:1 |
| 4341 | Metaling Card | EQP_WEAPON | bonus3 bAutoSpell,RG_STRIPWEAPON,1,50; | METALING:1, E_METALING:1, C2_METALING:1 +1 |
| 4342 | Rsx 0806 Card | EQP_ARMOR | bonus bVit,3; bonus bUnbreakableArmor,0; bonus bNoKnockback,0; | RSX_0806:1 |
| 4343 | Mole Card | EQP_HEAD_LOW | bonus bLuk,2; | MOLE:1, C2_MOLE:1 |
| 4344 | Anopheles Card | EQP_ACC | bonus3 bAddMonsterDropItem,12058,RC_Insect,50; | ANOPHELES:1, E_ANOPHELES:1, E_ANOPHELES_:1 |
| 4345 | Hill Wind Card | EQP_WEAPON | bonus2 bSkillAtk,MG_THUNDERSTORM,5; bonus2 bSkillAtk,WZ_JUPITEL,5; bonus2 bSkillAtk,WZ_VERMILION,5; | HILL_WIND:1, HILL_WIND_1:1, C5_HILL_WIND_1:1 +1 |
| 4346 | Ygnizem Card | EQP_ARMOR | bonus bStr,readparam(bInt)/18; | YGNIZEM:1 |
| 4347 | Armaia Card | EQP_ACC | bonus3 bAddMonsterDropItem,12053,RC_Fish,50; | ARMAIA:1 |
| 4348 | Whikebain Card | EQP_ACC | bonus3 bAutoSpell,RG_STRIPARMOR,1,50; | WHIKEBAIN:1 |
| 4349 | Erend Card | EQP_ACC | bonus4 bAutoSpellWhenHit,AL_PNEUMA,1,50,0; | EREND:1 |
| 4350 | Rawrel Card | EQP_WEAPON | bonus2 bSkillAtk,WZ_FROSTNOVA,3; bonus2 bSkillAtk,WZ_STORMGUST,3; | RAWREL:1, C3_RAWREL:1 |
| 4351 | Kavac Card | EQP_GARMENT |  | KAVAC:1 |
| 4352 | B Ygnizem Card | EQP_SHOES | bonus bMaxHPrate,10; bonus bMaxSPrate,10; bonus2 bHPRegenRate,50,10000; bonus2 bSPRegenRate,10,10000; | B_YGNIZEM:1 |
| 4353 | Removal Card | EQP_ARMOR | bonus bMaxHP,800-40*getrefine(); bonus bHPrecovRate,10; | REMOVAL:1, C3_REMOVAL:1 |
| 4354 | Gemini Card | EQP_HEAD_LOW |  | GEMINI:1 |
| 4355 | Gremlin Card | EQP_ACC | bonus3 bAddMonsterDropItem,12043,RC_Brute,50; | GREMLIN:1 |
| 4356 | Beholder Card | EQP_ACC | skill SA_CASTCANCEL,1; | BEHOLDER:1 |
| 4357 | B Seyren Card | EQP_HEAD_LOW | skill LK_BERSERK,1; bonus bMaxHPrate,-50; | B_SEYREN:1 |
| 4358 | Seyren Card | EQP_HEAD_LOW | bonus bStr,getrefine()-6; | SEYREN:1 |
| 4359 | B Eremes Card | EQP_GARMENT | skill AS_CLOAKING,3; | B_EREMES:1, E_B_EREMES:1 |
| 4360 | Eremes Card | EQP_WEAPON | bonus2 bCriticalAddRace,RC_DemiPlayer,10; | EREMES:1 |
| 4361 | B Harword Card | EQP_WEAPON | bonus bBreakWeaponRate,1000; bonus bBreakArmorRate,700; | B_HARWORD:1, E_B_HARWORD:1 |
| 4362 | Harword Card | EQP_WEAPON | bonus bAspdRate,-5; bonus bHit,30; | HARWORD:1 |
| 4363 | B Magaleta Card | EQP_ARMOR | bonus5 bAutoSpellWhenHit,HP_ASSUMPTIO,1,50,BF_WEAPON-BF_MAGIC,0; | B_MAGALETA:1 |
| 4364 | Magaleta Card | EQP_HEAD_LOW | bonus bInt,1; bonus5 bAutoSpellWhenHit,PR_LEXDIVINA,5,150,BF_MAGIC,1; | MAGALETA:1 |
| 4365 | B Katrinn Card | EQP_HEAD_LOW | bonus bIgnoreMdefRace,RC_NonBoss; bonus bVariableCastrate,100; bonus bSPrecovRate,-100; | B_KATRINN:1 |
| 4366 | Katrinn Card | EQP_HEAD_LOW |  | KATRINN:1 |
| 4367 | B Shecil Card | EQP_WEAPON | bonus2 bHPDrainRate,10,20; bonus bHPrecovRate,-10; | B_SHECIL:1 |
| 4368 | Shecil Card | EQP_WEAPON | bonus bAspdRate,5; bonus bHit,-30; | SHECIL:1, C2_SHECIL:1 |
| 4369 | Venatu Card | EQP_ARMOR | bonus bLuk,readparam(bAgi)/18; | VENATU:1, VENATU_1:1, VENATU_2:1 +3 |
| 4370 | Dimik Card | EQP_ARMOR | bonus bVit,getrefine()-5; | DIMIK:1, DIMIK_1:1, DIMIK_2:1 +4 |
| 4371 | Archdam Card | EQP_ARMOR | bonus bBaseAtk,10; bonus bVariableCastrate,20; | ARCHDAM:1 |
| 4372 | Bacsojin Card | EQP_HEAD_LOW | bonus bHealPower,30; bonus bUseSPrate,15; | BACSOJIN_:1 |
| 4373 | Chung E Card | EQP_GARMENT | bonus bLuk,getrefine()-5; bonus bCritical,min(getrefine(),10); | CHUNG_E_:1 |
| 4374 | Apocalips H Card | EQP_HEAD_LOW | bonus bDex,2; bonus2 bIgnoreMdefRate,RC_Boss,30; | APOCALIPS_H:1, E_APOCALIPS_H:1 |
| 4375 | Orc Baby Card Card | EQP_GARMENT |  | ORC_BABY:1 |
| 4376 | Lady Tanee Card | EQP_SHOES | bonus bMaxHPrate,-40; bonus bMaxSPrate,50; bonus2 bAddMonsterDropItem,513,200; bonus2 bAddItemHealRate,513,100; | LADY_TANEE:1 |
| 4377 | Green Iguana Card | EQP_ACC | bonus3 bAddMonsterDropItem,12063,RC_Formless,50; | GREEN_IGUANA:1, C5_GREEN_IGUANA:1 |
| 4378 | Acidus Card | EQP_SHOES |  | ACIDUS:1 |
| 4379 | Acidus  Card | EQP_HEAD_LOW |  | ACIDUS_:1, C2_ACIDUS_:1, C3_ACIDUS_:1 |
| 4380 | Ferus Card | EQP_WEAPON | bonus2 bSkillAtk,WZ_FIREPILLAR,5; bonus2 bSkillAtk,WZ_METEOR,5; | FERUS:1 |
| 4381 | Ferus  Card | EQP_SHOES | bonus bVit,1; bonus bMaxHPrate,10; | FERUS_:1, C4_FERUS_:1 |
| 4382 | Novus  Card | EQP_ARMOR | bonus bMaxHP,500; bonus bHPrecovRate,10; | NOVUS_:1 |
| 4383 | Novus Card | EQP_ARMOR | bonus3 bAddEffWhenHit,Eff_Confusion,3000,ATF_TARGET-ATF_SELF; | NOVUS:1, C5_NOVUS:1, C1_NOVUS:1 +2 |
| 4384 | Hydro Card | EQP_ACC | bonus3 bAutoSpell,SA_SPELLBREAKER,1,100; | HYDRO:1 |
| 4385 | Dragon Egg Card | EQP_ACC | bonus3 bAddMonsterDropItem,12048,RC_Dragon,50; | DRAGON_EGG:1 |
| 4386 | Detale Card | EQP_ARMOR | bonus bMdef,-20; bonus2 bResEff,Eff_Freeze,10000; bonus5 bAutoSpellWhenHit,SA_LANDPROTECTOR,1,70,BF_MAGIC,0; | DETALE:1 |
| 4387 | Ancient Mimic Card | EQP_ARMOR | bonus bAgi,readparam(bLuk)/18; | ANCIENT_MIMIC:1, C3_ANCIENT_MIMIC:1 |
| 4388 | Deathword Card | EQP_WEAPON | bonus2 bSkillAtk,MG_NAPALMBEAT,5; bonus2 bSkillAtk,MG_SOULSTRIKE,5; bonus2 bSkillAtk,HW_NAPALMVULCAN,5; | DEATHWORD:1, C1_DEATHWORD:1, C2_DEATHWORD:1 +1 |
| 4389 | Plasma Card | EQP_ACC | bonus2 bAddMonsterDropItem,12118,50; bonus2 bAddMonsterDropItem,12119,50; bonus2 bAddMonsterDropItem,12120,50; bonus2 bA | PLASMA_Y:1, PLASMA_R:1, PLASMA_G:1 +3 |
| 4390 | Breeze Card | EQP_WEAPON | bonus bBaseAtk,5; bonus2 bAddEff,Eff_Bleeding,500; | BREEZE:1, C3_BREEZE:1, C4_BREEZE:1 |
| 4391 | Retribution Card | EQP_ACC | bonus3 bAddMonsterDropItem,12068,RC_Angel,50; | RETRIBUTION:1, C4_RETRIBUTION:1, C5_RETRIBUTION:1 +1 |
| 4392 | Observation Card | EQP_ARMOR | bonus bDex,readparam(bVit)/18; | OBSERVATION:1, C3_OBSERVATION:1 |
| 4393 | Shelter Card | EQP_ARMOR | bonus bInt,readparam(bStr)/18; | SHELTER:1, C5_SHELTER:1 |
| 4394 | Solace Card | EQP_WEAPON | if(BaseJob==Job_Priest) bonus3 bAutoSpell,CR_GRANDCROSS,5,20; | SOLACE:1, C5_SOLACE:1 |
| 4395 | Tha Maero Card | EQP_WEAPON | bonus bBaseAtk,5; bonus3 bAutoSpell,AL_DECAGI,3,50; | THA_MAERO:1 |
| 4396 | Tha Odium Card | EQP_SHOES | bonus bAgi,getrefine()-5; | THA_ODIUM:1 |
| 4397 | Tha Despero Card | EQP_SHIELD | bonus bInt,getrefine()-6; | THA_DESPERO:1 |
| 4398 | Tha Dolor Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Angel,10; | THA_DOLOR:1 |
| 4399 | Thanatos Card | EQP_WEAPON | bonus bDefRatioAtkRace, RC_All; bonus bSPDrainValue,-1; bonus bDef,-30; bonus bFlee,-30; | THANATOS:1 |
| 4400 | Aliza Card | EQP_ARMOR | bonus3 bAutoSpellWhenHit,DC_WINKCHARM,1,50+50*(BaseJob==Job_Dancer); | ALIZA:1, C3_ALIZA:1 |
| 4401 | Alicel Card | EQP_ARMOR | bonus bFlee,10; bonus bDef,-5; | ALICEL:1, C4_ALICEL:1 |
| 4402 | Aliot Card | EQP_GARMENT |  | ALIOT:1 |
| 4403 | Kiel Card | EQP_HEAD_LOW | bonus bDelayrate,-30; | KIEL_:1 |
| 4404 | Skogul Card | EQP_ARMOR | bonus3 bAddEffWhenHit,Eff_Bleeding,3000,ATF_TARGET-ATF_SELF; | SKOGUL:1, C2_SKOGUL:1 |
| 4405 | Frus Card | EQP_ARMOR | bonus bMagicDamageReturn,min(getrefine(),10)*2; if(BaseClass==Job_Mage) bonus bMdef,3; | FRUS:1 |
| 4406 | Skeggiold Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Demon,2; | SKEGGIOLD:1, SKEGGIOLD_:1 |
| 4407 | Randgris Card | EQP_WEAPON | bonus bUnbreakableWeapon,0; bonus2 bAddRace, RC_All, 10; bonus3 bAutoSpell,SA_DISPELL,1,50; | RANDGRIS:1, E_RANDGRIS:1 |
| 4408 | Gloom Under Night Card | EQP_ARMOR | bonus2 bAddEle,Ele_Holy,40; bonus2 bAddEle,Ele_Dark,40; bonus2 bAddRace,RC_Angel,40; bonus2 bAddRace,RC_Demon,40; | GLOOMUNDERNIGHT:1 |
| 4409 | Agav Card | EQP_ARMOR | bonus bMatkRate,5; bonus bDef,-10; if(BaseClass==Job_Mage) bonus bMaxSP,100; | AGAV:1, C1_AGAV:1 |
| 4410 | Echio Card | EQP_ARMOR | bonus bBaseAtk,15; if(BaseClass==Job_Swordman) bonus bMaxHP,500; | ECHIO:1, E_GLOOMUNDERNIGHT:1, C3_ECHIO:1 |
| 4411 | Vanberk Card | EQP_HEAD_LOW |  | VANBERK:1, C1_VANBERK:1 |
| 4412 | Isilla Card | EQP_HEAD_LOW |  | ISILLA:1, C3_ISILLA:1 |
| 4413 | Hodremlin Card | EQP_SHIELD |  | HODREMLIN:1, C3_HODREMLIN:1 |
| 4414 | Seeker Card | EQP_SHIELD | skill MG_STONECURSE,1; bonus2 bResEff,Eff_Stone,3000; bonus bMdef,10; | SEEKER:1 |
| 4415 | Snowier Card | EQP_ACC | bonus2 bAddMonsterDropItem,536,500; bonus2 bAddItemHealRate,536,100; | SNOWIER:1, C2_SNOWIER:1 |
| 4416 | Siroma Card | EQP_ACC | bonus2 bSkillAtk,MG_COLDBOLT,25; bonus2 bVariableCastrate,MG_COLDBOLT,-25; | SIROMA:1, C1_SIROMA:1 |
| 4417 | Ice Titan Card | EQP_SHOES |  | ICE_TITAN:1, C1_ICE_TITAN:1 |
| 4418 | Gazeti Card | EQP_ACC | bonus3 bAutoSpell,MG_COLDBOLT,2,100; | GAZETI:1 |
| 4419 | Ktullanux Card | EQP_ARMOR | bonus2 bAddEle,Ele_Fire,50; bonus5 bAutoSpellWhenHit,WZ_FROSTNOVA,10,20,BF_WEAPON-BF_MAGIC,0; | KTULLANUX:1, E_KTULLANUX:1 |
| 4420 | Muscipular Card | EQP_SHIELD | bonus3 bAutoSpellWhenHit,AL_HEAL,1,100; bonus3 bAutoSpellWhenHit,AL_INCAGI,1,100; | MUSCIPULAR:1, C5_MUSCIPULAR:1 |
| 4421 | Drosera Card | EQP_WEAPON | if(getiteminfo(getequipid(EQI_HAND_R),9)>3) bonus bCritical,15; | DROSERA:1, C2_DROSERA:1 |
| 4422 | Roween Card | EQP_GARMENT | bonus bFlee,5; bonus bFlee2,3; bonus2 bAddEle,Ele_Water,10; bonus2 bCriticalAddRace,RC_Fish,15; | ROWEEN:1, C2_ROWEEN:1 |
| 4423 | Galion Card | EQP_ACC | bonus bHit,5; bonus2 bAddEle,Ele_Water,5; | GALION:1, C5_GALION:1 |
| 4424 | Stapo Card | EQP_ACC | skill TF_PICKSTONE,1; skill TF_THROWSTONE,1; | STAPO:1, E_STAPO:1, C3_STAPO:1 |
| 4425 | Atroce Card | EQP_WEAPON |  | ATROCE:1 |
| 4426 | Byorgue Card | EQP_ARMOR |  | BYORGUE:1 |
| 4427 | Sword Guardian Card | EQP_WEAPON |  | SWORD_GUARDIAN:1 |
| 4428 | Bow Guardian Card | EQP_WEAPON |  | BOW_GUARDIAN:1 |
| 4429 | Salamander Card | EQP_GARMENT | bonus2 bSkillAtk,WZ_FIREPILLAR,40; bonus2 bSkillAtk,WZ_METEOR,40; | SALAMANDER:1, C5_SALAMANDER:1 |
| 4430 | Ifrit Card | EQP_ACC | bonus bBaseAtk,(JobLevel/10); bonus bCritical,(JobLevel/10); bonus bHit,(JobLevel/10); bonus3 bAutoSpellWhenHit,NPC_EART | IFRIT:1, E_IFRIT:1 |
| 4431 | Kasa Card | EQP_GARMENT | bonus3 bAutoSpell,MG_FIREBALL,5,20; bonus3 bAutoSpell,MG_FIREBOLT,5,20; | KASA:1, C5_KASA:1 |
| 4432 | Magmaring Card | EQP_GARMENT | bonus bBaseAtk,5; bonus2 bAddEle,Ele_Earth,10; bonus2 bCriticalAddRace,RC_Brute,15; | MAGMARING:1, E_MAGMARING:1, C1_MAGMARING:1 |
| 4433 | Imp Card | EQP_ACC | bonus2 bSkillAtk,MG_FIREBOLT,25; bonus2 bVariableCastrate,MG_FIREBOLT,-25; | IMP:1 |
| 4434 | Knocker Card | EQP_HEAD_LOW | bonus2 bAddRace,RC_Formless,5; bonus3 bAddMonsterDropItem,756,RC_Formless,10; bonus3 bAddMonsterDropItem,757,RC_Formless | KNOCKER:1, C3_KNOCKER:1 |
| 4435 | Zombie Slaughter Card | EQP_SHOES | bonus2 bAddRace,RC_DemiPlayer,1; bonus2 bMagicAddRace,RC_DemiPlayer,1; bonus bHPGainValue,50; | ZOMBIE_SLAUGHTER:1, C1_ZOMBIE_SLAUGHTER:1 |
| 4436 | Ragged Zombie Card | EQP_ACC | bonus2 bCriticalAddRace,RC_DemiPlayer,5; bonus2 bAddRace,RC_DemiPlayer,1; bonus2 bMagicAddRace,RC_DemiPlayer,1; bonus2 b | RAGGED_ZOMBIE:1 |
| 4437 | Hell Poodle Card | EQP_ACC | bonus bHit,1; bonus2 bAddItemHealRate,517,100; bonus3 bAddEff,Eff_Bleeding,50,ATF_SHORT; | HELL_POODLE:1 |
| 4438 | Banshee Card | EQP_HEAD_LOW |  | BANSHEE:1, C1_BANSHEE:1 |
| 4439 | Flame Skull Card | EQP_SHIELD | bonus2 bResEff,Eff_Blind,3000; bonus2 bResEff,Eff_Stun,3000; bonus2 bResEff,Eff_Curse,3000; bonus2 bResEff,Eff_Stone,300 | FLAME_SKULL:1 |
| 4440 | Necromancer Card | EQP_WEAPON |  | NECROMANCER:1, C3_NECROMANCER:1 |
| 4441 | Fallen Bishop Card | EQP_SHOES | bonus bMatkRate,10; bonus bMaxSPrate,-50; bonus2 bMagicAddRace,RC_Angel,50; bonus2 bMagicAddRace,RC_DemiPlayer,50; | FALLINGBISHOP:1, E_FALLINGBISHOP:1 |
| 4442 | Tatacho Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Neutral, 20, 3); bonus2(bAddEle, Ele_Neutral, 5); | TATACHO:1, C2_TATACHO:1 |
| 4443 | Aqua Elemental Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Water, 20, 3); bonus2(bAddEle, Ele_Water, 5); | AQUA_ELEMENTAL:1 |
| 4444 | Draco Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Earth, 20, 3); bonus2(bAddEle, Ele_Earth, 5); | DRACO:1, C2_DRACO:1 |
| 4445 | Luciola Vespa Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Wind, 20, 3); bonus2(bAddEle, Ele_Wind, 5); | LUCIOLA_VESPA:1, C5_LUCIOLA_VESPA:1 |
| 4446 | P Skeleton Card | EQP_WEAPON |  |  |
| 4447 | Centipede Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Poison, 20, 3); bonus2(bAddEle, Ele_Poison, 5); | CENTIPEDE:1, C1_CENTIPEDE:1 |
| 4448 | Cornus Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Holy, 20, 3); bonus2(bAddEle, Ele_Holy, 5); | CORNUS:1 |
| 4449 | Dark Shadow Card | EQP_SHIELD | bonus3(bSubDefEle, Ele_Dark, 20, 3); bonus2(bAddEle, Ele_Dark, 5); | DARK_SHADOW:1 |
| 4450 | Banshee Master Card | EQP_ARMOR | bonus bInt,1; bonus bMatk,10; | BANSHEE_MASTER:1, C5_BANSHEE_MASTER:1 |
| 4451 | Ant Buyanne Card | EQP_ARMOR | bonus bMatk,100; | ENTWEIHEN:1 |
| 4452 | Centipede Larva Card | EQP_WEAPON | bonus bInt,1; bonus bMatk,3; | CENTIPEDE_LARVA:1 |
| 4453 | Hilsrion Card | EQP_WEAPON | bonus bBaseAtk,25; | HILLSRION:1 |
| 4454 | Light Up Card1 | EQP_WEAPON |  |  |
| 4455 | Light Up Card2 | EQP_WEAPON |  |  |
| 4456 | Nidhogg Shadow Card | EQP_ARMOR | bonus bInt,5; if (Class == Job_High_Wizard -- Class == Job_Baby_Warlock -- Class == Job_Warlock -- Class == Job_Warlock_ | S_NYDHOG:1 |
| 4457 | Nahtzigger Card | EQP_ARMOR | bonus2 bSkillAtk,MG_NAPALMBEAT,30; bonus2 bSkillAtk,MG_SOULSTRIKE,30; bonus2 bSkillAtk,HW_NAPALMVULCAN,30; bonus2 bSkill | NAGHT_SIEGER:1 |
| 4458 | Duneirre Card | EQP_HEAD_LOW |  | DUNEYRR:1 |
| 4459 | Lata Card | EQP_HEAD_LOW |  | RATA:1 |
| 4460 | Ringco Card | EQP_HEAD_LOW | bonus bHealPower,4; bonus2 bSkillUseSP,AL_HEAL,-15; | RHYNCHO:1 |
| 4461 | Pillar Card | EQP_HEAD_LOW |  | PHYLLA:1 |
| 4462 | Hardrock Mommos Card | EQP_ARMOR |  | HARDROCK_MOMMOTH:1 |
| 4463 | Tendrilion Card | EQP_WEAPON |  | TENDRILRION:1 |
| 4464 | Aunoe Card | EQP_WEAPON | bonus bCritAtkRate,20; | AUNOE:1 |
| 4465 | Panat Card | EQP_WEAPON |  | FANAT:1 |
| 4466 | Beholder Master Card | EQP_WEAPON |  | BEHOLDER_MASTER:1 |
| 4467 | Heavy Metaling Card | EQP_SHOES |  | HEAVY_METALING:1 |
| 4468 | Pinguicula Dark Card | EQP_HEAD_LOW | bonus bBaseAtk,10; bonus2 bAddMonsterDropItem,7932,10; bonus2 bAddMonsterDropItem,7933,10; bonus2 bAddMonsterDropItem,79 | PINGUICULA_D:1, C2_PINGUICULA_D:1 |
| 4469 | Naga Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Fish,10; | NAGA:1 |
| 4470 | Nepenthes Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Plant,10; | NEPENTHES:1, C2_NEPENTHES:1 |
| 4471 | Egg Of Draco Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Dragon,10; | DRACO_EGG:1 |
| 4472 | Bradium Goram Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Brute,10; | BRADIUM_GOLEM:1, C5_BRADIUM_GOLEM:1 |
| 4473 | Ancient Tree Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Undead,10; | ANCIENT_TREE:1 |
| 4474 | Jakudam Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_DemiPlayer,10; | ZAKUDAM:1, C5_ZAKUDAM:1 |
| 4475 | Cobalt Mineral Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Formless,10; | COBALT_MINERAL:1 |
| 4476 | Pinguicula Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Insect,10; | PINGUICULA:1, C3_PINGUICULA:1 |
| 4477 | Hell Apocalips Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Demon,10; | HELL_APOCALIPS:1 |
| 4478 | Light Up Card3 | EQP_SHOES |  |  |
| 4479 | Light Up Card4 | EQP_GARMENT |  |  |
| 4480 | Sealed Kiel Card | EQP_HEAD_LOW | bonus bDelayrate,((getrefine()>14)?-20:-15); |  |
| 4481 | Sealed Ktullanux Card | EQP_ARMOR | bonus2 bAddEle,Ele_Fire,((getrefine()>14)?35:25); bonus5 bAutoSpellWhenHit,WZ_FROSTNOVA,10,10,BF_WEAPON-BF_MAGIC,0; |  |
| 4482 | Sealed B Ygnizem Card | EQP_SHOES | .@rate = (getrefine()>14)?7:5; bonus bMaxHPrate,.@rate; bonus bMaxSPrate,.@rate; bonus2 bHPRegenRate,50,10000; bonus2 bS |  |
| 4483 | Sealed Dracula Card | EQP_WEAPON | bonus2 bSPDrainRate,((getrefine()>14)?70:50),5; |  |
| 4484 | Sealed Mistress Card | EQP_HEAD_LOW | bonus bNoGemStone,0; bonus bUseSPrate,((getrefine()>14)?35:50); |  |
| 4485 | Sealed Gloom Card | EQP_ARMOR | .@rate = (getrefine()>14)?30:20; bonus2 bAddEle,Ele_Holy,.@rate; bonus2 bAddEle,Ele_Dark,.@rate; bonus2 bAddRace,RC_Ange |  |
| 4486 | Sealed Berz Card | EQP_ACC | bonus bVariableCastrate,-15; |  |
| 4487 | Sealed Ifrit Card | EQP_ACC | bonus bBaseAtk,(JobLevel/20); bonus bCritical,(JobLevel/20); bonus bHit,(JobLevel/20); bonus3 bAutoSpellWhenHit,NPC_EART |  |
| 4488 | Sealed D Lord Card | EQP_SHOES | bonus3 bAutoSpellWhenHit,WZ_METEOR,5,50; |  |
| 4489 | Sealed Pharaoh Card | EQP_HEAD_LOW | bonus bUseSPrate,-15; |  |
| 4490 | Sealed M Flower Card | EQP_SHOES | skill AL_INCAGI,((getrefine()>14)?5:1); |  |
| 4491 | Sealed B Shecil Card | EQP_WEAPON | bonus bHPrecovRate,-100; bonus2 bHPDrainRate,5,((getrefine()>14)?15:10); |  |
| 4492 | Sealed Orc Hero Card | EQP_HEAD_LOW | bonus bVit,3; bonus2 bResEff,Eff_Stun,((getrefine()>14)?6000:4000); |  |
| 4493 | Sealed Tao Card | EQP_ARMOR | bonus bMaxHPrate,((getrefine()>14)?75:50); bonus bDefRate,-50; bonus bMdefRate,-50; |  |
| 4494 | Sealed TurtleG Card | EQP_WEAPON | bonus2 bAddRace, RC_All, (getrefine() > 14) ? 15 : 10; bonus3 bAutoSpell,SM_MAGNUM,10,15; |  |
| 4495 | Sealed Amon Ra Card | EQP_SHOES | bonus bAllStats,1; bonus3 bAutoSpellWhenHit,PR_KYRIE,((getrefine()>14)?8:5),(15+35*(readparam(bInt)>=99)); |  |
| 4496 | Sealed Drake Card | EQP_WEAPON |  |  |
| 4497 | Sealed Knight WS Card | EQP_WEAPON | bonus3 bAutoSpell,WZ_STORMGUST,1,10; bonus2 bAddEff,Eff_Freeze, ((getrefine()>14)?1500:1000); |  |
| 4498 | Sealed Lady Tanee Card | EQP_SHOES | bonus bMaxHPrate,((getrefine()>14)?-50:-60); bonus bMaxSPrate,50; bonus2 bAddMonsterDropItem,513,100; bonus2 bAddItemHea |  |
| 4499 | Sealed Samurai Card | EQP_WEAPON | bonus bIgnoreDefRace,RC_NonBoss; bonus bHPrecovRate,-100; if (getrefine()>14) bonus2 bHPLossRate,777,8000; else bonus2 b |  |
| 4500 | Sealed Orc Load Card | EQP_ARMOR | bonus bShortWeaponDamageReturn,((getrefine()>14)?25:15); |  |
| 4501 | Sealed B Magaleta Card | EQP_ARMOR | bonus5 bAutoSpellWhenHit,HP_ASSUMPTIO,1,((getrefine()>14)?35:25),BF_WEAPON-BF_MAGIC,0; |  |
| 4502 | Sealed B Harword Card | EQP_WEAPON |  |  |
| 4503 | Sealed Apocalips H Card | EQP_HEAD_LOW | bonus bDex,2; bonus2 bIgnoreMdefRate,RC_Boss,((getrefine()>14)?25:15); |  |
| 4504 | Sealed Eddga Card | EQP_SHOES | bonus bMaxHPrate,((getrefine()>14)?-35:-50); |  |
| 4505 | Scaraba Card | EQP_ACC | bonus bMatk,20; bonus bMaxSPrate,-1; | HORN_SCARABA:1, HORN_SCARABA2:1, ANTLER_SCARABA:1 +4 |
| 4506 | Dolomedes Card | EQP_HEAD_LOW |  | DOLOMEDES:1, C3_DOLOMEDES:1 |
| 4507 | Q Scaraba Card | EQP_WEAPON | bonus2 bAddRace2,RC2_Scaraba,30; bonus2 bAddMonsterDropItem,12806,30; | QUEEN_SCARABA:1 |
| 4508 | Gold Scaraba Card | EQP_ACC | bonus bBaseAtk,20; bonus bMaxHPrate,-1; | I_HORN_SCARABA:1, I_HORN_SCARABA2:1, I_ANTLER_SCARABA:1 +1 |
| 4509 | Gold Q Scaraba Card | EQP_HEAD_LOW |  | I_QUEEN_SCARABA:1 |
| 4510 | Miming Card | EQP_WEAPON | bonus2 bAddEff,Eff_Deepsleep,500; | MIMING:1, C3_MIMING:1 |
| 4511 | Little Fatum Card | EQP_WEAPON | bonus3 bAddEff,Eff_Silence,500,ATF_SKILL; | LITTLE_FATUM:1, C4_LITTLE_FATUM:1 |
| 4512 | Parus Card | EQP_HEAD_LOW |  | PARUS:1 |
| 4513 | Angra Mantis Card | EQP_HEAD_LOW |  | C5_ANGRA_MANTIS:1, C1_ANGRA_MANTIS:1 |
| 4514 | Pom Spider Card | EQP_WEAPON | bonus2 bAddRace,RC_Undead,20; | POM_SPIDER:1 |
| 4515 | Alnoldi Card | EQP_SHIELD | bonus2 bAddRaceTolerance,RC_Plant,30; | ALNOLDI:1, C1_ALNOLDI:1 |
| 4516 | Comodo Card | EQP_ARMOR | bonus bDef,50; bonus bFlee,-25; | COMODO:1, C5_COMODO:1 |
| 4517 | Cendrawasih Card | EQP_HEAD_LOW |  | CENDRAWASIH:1 |
| 4518 | Banaspaty Card | EQP_WEAPON | bonus2 bAddEff,Eff_Burning,1500; | BANASPATY:1, C2_BANASPATY:1 |
| 4519 | Butoijo Card | EQP_WEAPON | bonus2 bAddRace,RC_Angel,20; | BUTOIJO:1 |
| 4520 | Leak Card | EQP_GARMENT | bonus bStr,3; bonus2 bAddEff,Eff_Confusion,5000; bonus3 bAddEff,Eff_Confusion,5000,ATF_SKILL; | LEAK:1 |
| 4521 | Sedora Card | EQP_WEAPON | bonus bCritAtkRate,15; | SEDORA:1, C4_SEDORA:1 |
| 4522 | Sropho Card | EQP_WEAPON | bonus4 bAddEff,Eff_Cold,500,ATF_SHORT,3000; | SROPHO:1 |
| 4523 | Pot Dofle Card | EQP_ARMOR | bonus bDefEle,Ele_Water; bonus2 bAddRaceTolerance,RC_Fish,10; | POT_DOFLE:1 |
| 4524 | King Dramoh Card | EQP_HEAD_LOW |  | KING_DRAMOH:1 |
| 4525 | Kraken Card | EQP_GARMENT | bonus bFlee,10; skill TF_HIDING,1; skill RG_RAID,1; bonus3 bAddEffOnSkill,RG_RAID,Eff_Bleeding,1000; | KRAKEN:1 |
| 4526 | Odd Coelacanth Card | EQP_ARMOR | bonus bMaxSPrate,5; bonus bMdef,50; | COELACANTH_N_M:1 |
| 4527 | Black Coelacanth Card | EQP_ARMOR | bonus bMaxHPrate,10; bonus bDef,100; | COELACANTH_N_A:1 |
| 4528 | Mutant Coelacanth Card | EQP_HEAD_LOW | bonus bMatkRate,2+(getrefine()/2); bonus bMaxHPrate,-getrefine()/2; | COELACANTH_H_M:1 |
| 4529 | Cruel Coelacanth Card | EQP_HEAD_LOW | bonus2 bAddRace, RC_All, 2+(getrefine()/2); bonus bMaxSPrate,-getrefine()/2; | COELACANTH_H_A:1 |
| 4530 | Siorava Card | EQP_HEAD_LOW |  | SIORAVA:1 |
| 4531 | Red Eruma Card | EQP_WEAPON | bonus3 bAddEff,Eff_Curse,500,ATF_SKILL; | RED_ERUMA:1, C5_RED_ERUMA:1 |
| 4532 | Wild Rider Card | EQP_SHOES | bonus3 bAutoSpellWhenHit,AL_INCAGI,1,10; | WILD_RIDER:1 |
| 4533 | Mini Octopus Card | EQP_WEAPON | bonus3 bAddEff,Eff_Blind,500,ATF_SKILL; | MD_OCTOPUS:1 |
| 4534 | Giant Octopus Card | EQP_ARMOR | bonus bMaxHPrate,12; skill WZ_WATERBALL,5; | MD_GIANT_OCTOPUS:1 |
| 4535 | Sealed Rand Card | EQP_WEAPON |  |  |
| 4536 | Sealed Atroce Card | EQP_WEAPON |  |  |
| 4537 | Sealed Phreeoni Card | EQP_WEAPON | bonus bHit,((getrefine()>14)?75:50); |  |
| 4538 | Sealed Bacsojin Card | EQP_HEAD_LOW |  |  |
| 4539 | Sealed F Bishop Card | EQP_SHOES | bonus bMatkRate,((getrefine()>14)?8:5); bonus bMaxSPrate,-50; .@rate = (getrefine()>14)?33:25; bonus2 bMagicAddRace,RC_A |  |
| 4540 | SLD Lord Of Death Card | EQP_WEAPON | .@rate = (getrefine()>14)?350:250; bonus3 bAddEff,Eff_Stun,.@rate,ATF_SHORT; bonus3 bAddEff,Eff_Curse,.@rate,ATF_SHORT;  |  |
| 4541 | SLD B Katrinn Card | EQP_HEAD_LOW |  |  |
| 4542 | SLD Detale Card | EQP_ARMOR |  |  |
| 4543 | SLD Garm Card | EQP_ARMOR | bonus2 bAddEffWhenHit,Eff_Freeze,((getrefine()>14)?4000:2500); |  |
| 4544 | SLD Dark Snake Card | EQP_HEAD_LOW |  |  |
| 4545 | Novice Poring Card | EQP_HEAD_LOW | bonus bLuk,1; | LITTLE_PORING:100, C3_LITTLE_PORING:100 |
| 4546 | Valkhiri Card | EQP_WEAPON |  |  |
| 4547 | Upd Byorgue Card | EQP_ARMOR |  |  |
| 4548 | Upd Salamander Card | EQP_GARMENT | bonus2 bSkillAtk,WZ_FIREPILLAR,40; bonus2 bSkillAtk,WZ_METEOR,40; |  |
| 4549 | Upd Maya Puple Card | EQP_HEAD_LOW | bonus bIntravision,0; bonus bAllStats,1; skill AL_RUWACH,1; |  |
| 4550 | Upd Bow Guardian Card | EQP_WEAPON |  |  |
| 4551 | Upd Necromancer Card | EQP_WEAPON |  |  |
| 4552 | Manny Card | EQP_ACC | bonus bMaxHP,10; |  |
| 4553 | Sid Card | EQP_ARMOR | bonus bMaxHP,100; |  |
| 4554 | Diego Card | EQP_GARMENT | bonus bMaxHP,100; |  |
| 4555 | Scrat Card | EQP_HEAD_LOW | bonus bMaxHP,100; |  |
| 4556 | Fenrir Card | EQP_HEAD_LOW | bonus bMatk,50; bonus bMatk,(getrefine()*5); bonus bFixedCastrate,-70; |  |
| 4557 | Fenrir Card  | EQP_HEAD_LOW | bonus bMatk,25; |  |
| 4558 | Woodie Card | EQP_GARMENT | bonus2 bSubEle,Ele_Earth,20; bonus3 bAutoSpellWhenHit,PR_KYRIE,2,20; | WOODIE:300 |
| 4559 | M Morocc Card | EQP_SHOES | bonus bAspd,1; bonus bMaxSPrate,-10; |  |
| 4560 | Clown Card | EQP_ARMOR |  | B_ALPHOCCIO:1 |
| 4561 | Professor Card | EQP_ARMOR |  | B_CELIA:1 |
| 4562 | Champion Card | EQP_ARMOR |  | B_CHEN:1 |
| 4563 | Creator Card | EQP_ARMOR |  | B_FLAMEL:1 |
| 4564 | Stalker Card | EQP_ARMOR |  | B_GERTIE:1 |
| 4565 | Paladin Card | EQP_ARMOR |  | B_RANDEL:1 |
| 4566 | Gypsy Card | EQP_ARMOR |  | B_TRENTINI:1 |
| 4567 | Alphoccio Card | EQP_GARMENT |  | ALPHOCCIO:1 |
| 4568 | Ceila Card | EQP_GARMENT | bonus bFlee,10; skill SA_ABRACADABRA,1; | CELIA:1, C3_CELIA:1 |
| 4569 | Chen Card | EQP_GARMENT | bonus bFlee,10; skill MO_CALLSPIRITS,2; | CHEN:1 |
| 4570 | Flamel Card | EQP_GARMENT | bonus bFlee,10; bonus2 bAddItemHealRate,501,200; bonus2 bAddItemHealRate,502,200; bonus2 bAddItemHealRate,503,200; bonus | FLAMEL:1 |
| 4571 | Gertie Card | EQP_GARMENT | bonus bFlee,10; skill RG_CLOSECONFINE,1; | GERTIE:1 |
| 4572 | Randel Card | EQP_GARMENT | bonus bFlee,10; skill CR_AUTOGUARD,3; | RANDEL:1 |
| 4573 | Trentini Card | EQP_GARMENT |  | TRENTINI:1 |
| 4574 | Daehyon Card | EQP_WEAPON | .@equip = getiteminfo(getequipid(EQI_HAND_R), 11); if (.@equip == 2 -- .@equip == 3) bonus(bBaseAtk, 100); | DAEHYON:1 |
| 4575 | Soheon Card | EQP_WEAPON |  | SOHEON:1 |
| 4576 | Gioia Card | EQP_GARMENT | bonus2 bMagicAtkEle,Ele_Wind,100; bonus2 bMagicAtkEle,Ele_Ghost,100; bonus2 bSubEle,Ele_Neutral,-30; bonus2 bSubEle,Ele_ | GIOIA:1 |
| 4577 | Elvira Card | EQP_ACC | bonus2 bMagicAtkEle,Ele_Wind,20; bonus2 bMagicAtkEle,Ele_Ghost,20; | ELVIRA:1 |
| 4578 | Pyuriel Card | EQP_WEAPON | bonus bCritAtkRate,30; bonus2 bSubRace, RC_All, -10; | PYURIEL:1 |
| 4579 | Lora Card | EQP_WEAPON |  | LORA:1 |
| 4580 | Kades Card | EQP_GARMENT | bonus2 bSubEle,Ele_Water,50; bonus2 bSubEle,Ele_Earth,50; bonus2 bSubEle,Ele_Fire,50; bonus2 bSubEle,Ele_Wind,50; bonus2 | KADES:1 |
| 4581 | Rudo Card | EQP_SHOES |  | RUDO:1 |
| 4582 | Bungisngis Card | EQP_HEAD_LOW | bonus bMaxHPrate,getrefine()/2; | BUNGISNGIS:1, C2_BUNGISNGIS:1 |
| 4583 | Engkanto Card | EQP_HEAD_LOW | bonus2 bAddEle,Ele_Poison,30; bonus2 bIgnoreDefRate,RC_Plant,30; | ENGKANTO:1 |
| 4584 | Manananggal Card | EQP_WEAPON | bonus bSPDrainValue,1; bonus bMaxSPrate,-1; | MANANANGGAL:1, C3_MANANANGGAL:1 |
| 4585 | Mangkukulam Card | EQP_ARMOR | bonus bMaxSPrate,10; bonus bHPGainValue,-666; | MANGKUKULAM:1 |
| 4586 | Tikbalang Card | EQP_HEAD_LOW |  | TIKBALANG:1 |
| 4587 | Tiyanak Card | EQP_ACC | bonus2 bCriticalAddRace,RC_DemiPlayer,12; bonus2 bCriticalAddRace,RC_Fish,12; bonus2 bCriticalAddRace,RC_Brute,12; | TIYANAK:1, C5_TIYANAK:1 |
| 4588 | Wakwak Card | EQP_GARMENT | bonus bBaseAtk,5*(readparam(bStr)/10); | WAKWAK:1 |
| 4589 | Jejeling Card | EQP_GARMENT | bonus bMaxHP,200*(readparam(bVit)/10); | JEJELING:1 |
| 4590 | Bangungot Card | EQP_ARMOR | bonus bInt,4; bonus5 bAutoSpellWhenHit,NPC_WIDESLEEP,3,2500,BF_MAGIC,0; | BANGUNGOT_1:1 |
| 4591 | Bakonawa Card | EQP_ARMOR | bonus bStr,4; bonus2 bAddEffWhenHit,Eff_Bleeding,2000; | BAKONAWA_1:1 |
| 4592 | Buwaya Card | EQP_ARMOR | bonus bVit,4; bonus5 bAutoSpellWhenHit,NPC_WIDESTONE,3,2500,BF_MAGIC,0; | BUWAYA:1 |
| 4593 | Menblatt Card | EQP_GARMENT | bonus bLongAtkRate,1*(readparam(bDex)/10); | MENBLATT:1, C5_MENBLATT:1 |
| 4594 | Petal Card | EQP_GARMENT | bonus bCritAtkRate,2*(readparam(bLuk)/10); | PETAL:1 |
| 4595 | Cenere Card | EQP_GARMENT | bonus bAspdRate,2*(readparam(bAgi)/10); bonus bDelayrate,-2*(readparam(bAgi)/10); | CENERE:1, C2_CENERE:1 |
| 4596 | AntiqueBook Card | EQP_GARMENT | bonus bMatk,5*(readparam(bInt)/10); | ANTIQUE_BOOK:1, C1_ANTIQUE_BOOK:1, C2_ANTIQUE_BOOK:1 |
| 4597 | LichternB Card | EQP_HEAD_LOW | bonus bMatk,10; bonus2 bMagicAtkEle,Ele_Water,(getrefine()>=9)?10:5; | LICHTERN_B:1 |
| 4598 | LichternY Card | EQP_HEAD_LOW | bonus bMatk,10; bonus2 bMagicAtkEle,Ele_Ghost,(getrefine()>=9)?10:5; | LICHTERN_G:1 |
| 4599 | LichternR Card | EQP_HEAD_LOW | bonus bMatk,10; bonus2 bMagicAtkEle,Ele_Fire,(getrefine()>=9)?10:5; | LICHTERN_R:1 |
| 4600 | LichternG Card | EQP_HEAD_LOW | bonus bMatk,10; bonus2 bMagicAtkEle,Ele_Earth,(getrefine()>=9)?10:5; | LICHTERN_Y:1 |
| 4601 | Amdarais Card | EQP_ARMOR | bonus bAtkRate,15; bonus bMatkRate,15; bonus2 bHPLossRate,666,4000; bonus2 bSPLossRate,66,4000; | MG_AMDARAIS:1 |
| 4602 | AmdaraisH Card | EQP_ARMOR | bonus bAtkRate,20; bonus bMatkRate,20; bonus2 bHPLossRate,666,6000; bonus2 bSPLossRate,66,6000; |  |
| 4603 | CorruptionRoot Card | EQP_WEAPON | bonus bBaseAtk,20; bonus5 bAutoSpellWhenHit,NPC_WIDESTONE,1,70,BF_WEAPON,0; bonus5 bAutoSpellWhenHit,NPC_WIDESLEEP,1,70, | MG_CORRUPTION_ROOT:1 |
| 4604 | CorruptionRootH Card | EQP_WEAPON | bonus bBaseAtk,30; bonus5 bAutoSpellWhenHit,NPC_WIDESTONE,2,70,BF_WEAPON,0; bonus5 bAutoSpellWhenHit,NPC_WIDESLEEP,2,70, |  |
| 4605 | UndeadKnightM Card | EQP_ARMOR | bonus bMaxHPrate,-44; bonus bHPGainValue,200+10*getrefine(); | MG_M_UNDEAD_KNIGHT:1 |
| 4606 | UndeadKnightF Card | EQP_GARMENT | bonus bMaxSPrate,-44; bonus bSPGainValue,20+(getrefine()/2); | MG_F_UNDEAD_KNIGHT:1 |
| 4607 | FaithfulManager Card | EQP_WEAPON |  | FAITHFUL_MANAGER:1 |
| 4608 | White Knightage Card | EQP_WEAPON | bonus bBaseAtk,15; bonus2 bAddSize,Size_Medium,20; bonus2 bAddSize,Size_Large,20; |  |
| 4609 | Khali Knightage Card | EQP_SHIELD | bonus bDef,20; bonus2 bSubSize,Size_Medium,25; bonus2 bSubSize,Size_Large,25; |  |
| 4610 | Sarah Card | EQP_ARMOR |  | MM_SARAH:1 |
| 4625 | Timeholder Card | EQP_WEAPON | bonus bMatkRate,20; bonus bUseSPrate,10; | TIMEHOLDER:1 |
| 4626 | Big Ben Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Formless,5; bonus2 bMagicAddRace,RC_Demon,5; | BIG_BEN:1 |
| 4627 | Big Bell Card | EQP_WEAPON | bonus2 bAddRace,RC_Formless,10; bonus2 bAddRace,RC_Demon,10; | BIG_BELL:1 |
| 4628 | Neo Punk Card | EQP_SHIELD | bonus2 bSubRace,RC_Formless,20; bonus2 bSubRace,RC_Demon,20; | NEO_PUNK:1 |
| 4629 | Arc Elder Card | EQP_GARMENT | bonus2 bSubEle,Ele_Neutral,15; bonus2 bMagicAtkEle,Ele_Earth,(getrefine()*3); | ARC_ELDER:1 |
| 4630 | Time Keeper Card | EQP_SHOES | bonus3 bAutoSpell,NPC_WIDECURSE,2,20; | TIME_KEEPER:1 |
| 4631 | Owl Viscount Card | EQP_ACC | bonus bAspdRate,3; | OWL_VISCOUNT:1 |
| 4632 | Owl Marquees Card | EQP_ACC |  | OWL_MARQUEES:1 |
| 4633 | P Archer Skeleton Card | EQP_WEAPON |  |  |
| 4634 | P Soldier Skeleton Card | EQP_WEAPON |  |  |
| 4635 | P Amdarais Card | EQP_ARMOR |  |  |
| 4636 | Bijou Card | EQP_SHIELD | bonus2 bResEff,Eff_Freeze,10000; bonus bAtkRate,10; bonus bMatkRate,10; |  |
| 4637 | Immortal Corpse Card | EQP_GARMENT | bonus bHPGainValue,50; bonus bSPGainValue,5; bonus bHPrecovRate,-100; |  |
| 4638 | Watcher Card | EQP_ARMOR | bonus bAtk,30; |  |
| 4639 | Taffy Card | EQP_ACC | bonus bAtkRate,1; |  |
| 4640 | Frozen Wolf Card | EQP_ACC | bonus bMatkRate,1; |  |
| 4641 | Zombie Guard Card | EQP_SHIELD | bonus bSPrecovRate,-100; |  |
| 4642 | Min Toad Card | EQP_SHOES | bonus bFlee2,2; if (getrefine() > 6) bonus bFlee2,2; if (getrefine() > 8) bonus bFlee2,3; |  |
| 4643 | Min Vagabond Wolf Card | EQP_SHOES | bonus bAtk,10; if (getrefine() > 6) bonus bAtk,10; if (getrefine() > 8) bonus bAtk,15; |  |
| 4644 | Min Vocal Card | EQP_SHOES | bonus bMdef,5; if (getrefine() > 6) bonus bMdef,10; if (getrefine() > 8) bonus bMdef,15; |  |
| 4645 | Min Eclipse Card | EQP_SHOES | bonus bMaxHP,300; if (getrefine() > 6) bonus bMaxHP,300; if (getrefine() > 8) bonus bMaxHP,400; |  |
| 4646 | Min Chimera Card | EQP_GARMENT | bonus bMaxHPrate,8; bonus bMaxSPrate,4; |  |
| 4647 | Min Osiris Card | EQP_ACC | bonus bHPGainValue,300; |  |
| 4648 | Min Eddga Card | EQP_SHOES | bonus3 bAutoSpellWhenHit,SM_PROVOKE,10,500; |  |
| 4649 | Min Phreeoni Card | EQP_WEAPON | bonus bCritical,100; |  |
| 4650 | Min Ork Hero Card | EQP_HEAD_LOW | bonus bVit,3; bonus3 bAddEffWhenHit,Eff_Stun,10000,BF_MAGIC; |  |
| 4651 | Min Tao Gunka Card | EQP_ARMOR | bonus bMaxHP,10000; bonus bAgi,-10; |  |
| 4652 | N Amon Ra Card | EQP_ARMOR | bonus2 bMagicAddEle,Ele_Dark,50; bonus2 bMagicAddEle,Ele_Undead,50; bonus2 bMagicAddRace,RC_Demon,50; bonus2 bMagicAddRa |  |
| 4653 | N Arclouse Card | EQP_SHIELD | bonus2 bSubRace,RC_Brute,20; bonus2 bSubRace,RC_Undead,20; |  |
| 4654 | N Mimic Card | EQP_WEAPON | bonus2 bMagicAddRace,RC_Brute,5; bonus2 bMagicAddRace,RC_Undead,5; |  |
| 4655 | N Minorous Card | EQP_WEAPON | bonus2 bAddRace,RC_Brute,10; bonus2 bAddRace,RC_Undead,10; |  |
| 4656 | N Mummy Card | EQP_SHOES |  |  |
| 4657 | N Ancient Mummy Card | EQP_GARMENT | bonus2 bSubEle,Ele_Neutral,15; bonus2 bMagicAtkEle,Ele_Fire,3+(getrefine()*3); |  |
| 4658 | N Verit Card | EQP_SHOES | bonus bMatkRate,5; if (getrefine() > 6) bonus bMatkRate,3; if (getrefine() > 8) bonus bMatkRate,2; |  |
| 4659 | Eggring Card | EQP_ARMOR | bonus bLuk,2; bonus bMaxHP,50; | DR_EGGRING:20 |
| 4660 | Scout Basilisk Card | EQP_SHIELD | bonus2 bSubSize,Size_Small,5; bonus2 bSubSize,Size_Medium,5; |  |
| 4661 | Charge Basilisk Card | EQP_SHIELD | bonus2 bSubSize,Size_Small,-15; bonus2 bSubSize,Size_Medium,20; bonus2 bSubSize,Size_Large,20; |  |
| 4662 | Big Eggring Card | EQP_GARMENT | bonus bAtk,25; bonus bMatk,25; bonus bAspdRate,10; bonus bMaxHP,1000; bonus bLongAtkRate,5; bonus bCritAtkRate,10; bonus |  |
| 4663 | Leaf Lunatic Card | EQP_SHOES | bonus bMaxSP,5; | DR_LUNATIC:10 |
| 4664 | Grass Fabre Card | EQP_ARMOR | bonus bLuk,1; bonus bMaxHP,100; |  |
| 4665 | Wild Hornet Card | EQP_WEAPON | bonus bAtk,5; |  |
| 4666 | Sweet Rodafrog Card | EQP_ARMOR | bonus bMaxSP,10; bonus bMaxHP,300; |  |
| 4667 | Hunter Wolf Card | EQP_SHOES | bonus bMaxSP,30; |  |
| 4668 | Trance Spore Card | EQP_HEAD_LOW | bonus bVit,1; bonus bInt,1; |  |
| 4669 | Jungle Mandragora Card | EQP_WEAPON | bonus2 bMagicAddEle,Ele_Wind,3; if (getrefine() > 6) bonus2 bMagicAddEle,Ele_Wind,5; if (getrefine() > 8) bonus2 bMagicA |  |
| 4670 | Fruit Pom Spider Card | EQP_WEAPON | bonus2 bMagicAddEle,Ele_Fire,3; if (getrefine() > 6) bonus2 bMagicAddEle,Ele_Fire,5; if (getrefine() > 8) bonus2 bMagicA |  |
| 4671 | V Celia Card | EQP_GARMENT |  |  |
| 4672 | V Chen Card | EQP_GARMENT |  |  |
| 4673 | V Alphoccio Card | EQP_GARMENT |  |  |
| 4674 | V Eremes Card | EQP_GARMENT |  |  |
| 4675 | V Magaleta Card | EQP_GARMENT |  |  |
| 4676 | V Shecil Card | EQP_GARMENT |  |  |
| 4677 | V Harword Card | EQP_GARMENT |  |  |
| 4678 | V Katrinn Card | EQP_GARMENT |  |  |
| 4679 | V Seyren Card | EQP_GARMENT |  |  |
| 4680 | V Randel Card | EQP_GARMENT |  |  |
| 4681 | V Flamel Card | EQP_GARMENT |  |  |
| 4682 | V Gertie Card | EQP_GARMENT |  |  |
| 4683 | V Trentini Card | EQP_GARMENT |  |  |
| 4684 | V B Eremes Card | EQP_WEAPON | bonus2 bSkillAtk,GC_CROSSIMPACT,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,GC_CROSSIMPACT,50; if (getre |  |
| 4685 | V B Magaleta Card | EQP_WEAPON | bonus2 bSkillAtk,AB_JUDEX,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,AB_JUDEX,50; if (getrefine() >= 10 |  |
| 4686 | V B Katrinn Card | EQP_WEAPON | bonus2 bSkillAtk,WL_HELLINFERNO,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,WL_HELLINFERNO,50; if (getre |  |
| 4687 | V B Shecil Card | EQP_WEAPON | bonus2 bSkillAtk,RA_AIMEDBOLT,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,RA_AIMEDBOLT,50; if (getrefine |  |
| 4688 | V B Harword Card | EQP_WEAPON | bonus2 bSkillAtk,NC_POWERSWING,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,NC_POWERSWING,50; if (getrefi |  |
| 4689 | V B Seyren Card | EQP_WEAPON | bonus2 bSkillAtk,RK_SONICWAVE,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,RK_SONICWAVE,50; if (getrefine |  |
| 4690 | V B Randel Card | EQP_WEAPON | bonus2 bSkillAtk,LG_BANISHINGPOINT,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,LG_BANISHINGPOINT,50; if  |  |
| 4691 | V B Flamel Card | EQP_WEAPON | bonus2 bSkillAtk,GN_CARTCANNON,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,GN_CARTCANNON,50; if (getrefi |  |
| 4692 | V B Celia Card | EQP_WEAPON | bonus2 bSkillAtk,SO_CLOUD_KILL,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,SO_CLOUD_KILL,50; if (getrefi |  |
| 4693 | V B Chen Card | EQP_WEAPON | bonus2 bSkillAtk,SR_DRAGONCOMBO,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,SR_DRAGONCOMBO,50; if (getre |  |
| 4694 | V B Gertie Card | EQP_WEAPON | bonus2 bSkillAtk,SC_FEINTBOMB,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,SC_FEINTBOMB,50; if (getrefine |  |
| 4695 | V B Trentini Card | EQP_WEAPON | bonus2 bSkillAtk,WM_METALICSOUND,50; if (getequipweaponlv(EQI_HAND_R) == 4) bonus2 bSkillAtk,WM_METALICSOUND,50; if (get |  |
| 4696 | V B Alphoccio Card | EQP_WEAPON |  |  |
| 4700 | Strength1 | EQP_HELM | bonus bStr,1; |  |
| 4701 | Strength2 | EQP_HELM | bonus bStr,2; |  |
| 4702 | Strength3 | EQP_HELM | bonus bStr,3; |  |
| 4703 | Strength4 | EQP_HELM | bonus bStr,4; |  |
| 4704 | Strength5 | EQP_HELM | bonus bStr,5; |  |
| 4705 | Strength6 | EQP_HELM | bonus bStr,6; |  |
| 4706 | Strength7 | EQP_HELM | bonus bStr,7; |  |
| 4707 | Strength8 | EQP_HELM | bonus bStr,8; |  |
| 4708 | Strength9 | EQP_HELM | bonus bStr,9; |  |
| 4709 | Strength10 | EQP_HELM | bonus bStr,10; |  |
| 4710 | Inteligence1 | EQP_HELM | bonus bInt,1; |  |
| 4711 | Inteligence2 | EQP_HELM | bonus bInt,2; |  |
| 4712 | Inteligence3 | EQP_HELM | bonus bInt,3; |  |
| 4713 | Inteligence4 | EQP_HELM | bonus bInt,4; |  |
| 4714 | Inteligence5 | EQP_HELM | bonus bInt,5; |  |
| 4715 | Inteligence6 | EQP_HELM | bonus bInt,6; |  |
| 4716 | Inteligence7 | EQP_HELM | bonus bInt,7; |  |
| 4717 | Inteligence8 | EQP_HELM | bonus bInt,8; |  |
| 4718 | Inteligence9 | EQP_HELM | bonus bInt,9; |  |
| 4719 | Inteligence10 | EQP_HELM | bonus bInt,10; |  |
| 4720 | Dexterity1 | EQP_HELM | bonus bDex,1; |  |
| 4721 | Dexterity2 | EQP_HELM | bonus bDex,2; |  |
| 4722 | Dexterity3 | EQP_HELM | bonus bDex,3; |  |
| 4723 | Dexterity4 | EQP_HELM | bonus bDex,4; |  |
| 4724 | Dexterity5 | EQP_HELM | bonus bDex,5; |  |
| 4725 | Dexterity6 | EQP_HELM | bonus bDex,6; |  |
| 4726 | Dexterity7 | EQP_HELM | bonus bDex,7; |  |
| 4727 | Dexterity8 | EQP_HELM | bonus bDex,8; |  |
| 4728 | Dexterity9 | EQP_HELM | bonus bDex,9; |  |
| 4729 | Dexterity10 | EQP_HELM | bonus bDex,10; |  |
| 4730 | Agility1 | EQP_HELM | bonus bAgi,1; |  |
| 4731 | Agility2 | EQP_HELM | bonus bAgi,2; |  |
| 4732 | Agility3 | EQP_HELM | bonus bAgi,3; |  |
| 4733 | Agility4 | EQP_HELM | bonus bAgi,4; |  |
| 4734 | Agility5 | EQP_HELM | bonus bAgi,5; |  |
| 4735 | Agility6 | EQP_HELM | bonus bAgi,6; |  |
| 4736 | Agility7 | EQP_HELM | bonus bAgi,7; |  |
| 4737 | Agility8 | EQP_HELM | bonus bAgi,8; |  |
| 4738 | Agility9 | EQP_HELM | bonus bAgi,9; |  |
| 4739 | Agility10 | EQP_HELM | bonus bAgi,10; |  |
| 4740 | Vitality1 | EQP_HELM | bonus bVit,1; |  |
| 4741 | Vitality2 | EQP_HELM | bonus bVit,2; |  |
| 4742 | Vitality3 | EQP_HELM | bonus bVit,3; |  |
| 4743 | Vitality4 | EQP_HELM | bonus bVit,4; |  |
| 4744 | Vitality5 | EQP_HELM | bonus bVit,5; |  |
| 4745 | Vitality6 | EQP_HELM | bonus bVit,6; |  |
| 4746 | Vitality7 | EQP_HELM | bonus bVit,7; |  |
| 4747 | Vitality8 | EQP_HELM | bonus bVit,8; |  |
| 4748 | Vitality9 | EQP_HELM | bonus bVit,9; |  |
| 4749 | Vitality10 | EQP_HELM | bonus bVit,10; |  |
| 4750 | Luck1 | EQP_HELM | bonus bLuk,1; |  |
| 4751 | Luck2 | EQP_HELM | bonus bLuk,2; |  |
| 4752 | Luck3 | EQP_HELM | bonus bLuk,3; |  |
| 4753 | Luck4 | EQP_HELM | bonus bLuk,4; |  |
| 4754 | Luck5 | EQP_HELM | bonus bLuk,5; |  |
| 4755 | Luck6 | EQP_HELM | bonus bLuk,6; |  |
| 4756 | Luck7 | EQP_HELM | bonus bLuk,7; |  |
| 4757 | Luck8 | EQP_HELM | bonus bLuk,8; |  |
| 4758 | Luck9 | EQP_HELM | bonus bLuk,9; |  |
| 4759 | Luck10 | EQP_HELM | bonus bLuk,10; |  |
| 4760 | Matk1 | EQP_HELM | bonus bMatkRate,1; bonus bFixedCastrate,-1; |  |
| 4761 | Matk2 | EQP_HELM | bonus bMatkRate,2; bonus bFixedCastrate,-1; |  |
| 4762 | Evasion6 | EQP_HELM | bonus bFlee,6; |  |
| 4763 | Evasion12 | EQP_HELM | bonus bFlee,12; |  |
| 4764 | Critical5 | EQP_HELM | bonus bCritical,5; |  |
| 4765 | Critical7 | EQP_HELM | bonus bCritical,7; |  |
| 4766 | Atk2 | EQP_HELM | bonus bAtkRate,2; |  |
| 4767 | Atk3 | EQP_HELM | bonus bAtkRate,3; |  |
| 4768 | Str1 J | EQP_HELM | bonus bStr,1; |  |
| 4769 | Str2 J | EQP_HELM | bonus bStr,2; |  |
| 4770 | Str3 J | EQP_HELM | bonus bStr,3; |  |
| 4771 | Int1 J | EQP_HELM | bonus bInt,1; |  |
| 4772 | Int2 J | EQP_HELM | bonus bInt,2; |  |
| 4773 | Int3 J | EQP_HELM | bonus bInt,3; |  |
| 4774 | Vit1 J | EQP_HELM | bonus bVit,1; |  |
| 4775 | Vit2 J | EQP_HELM | bonus bVit,2; |  |
| 4776 | Vit3 J | EQP_HELM | bonus bVit,3; |  |
| 4777 | Agi1 J | EQP_HELM | bonus bAgi,1; |  |
| 4778 | Agi2 J | EQP_HELM | bonus bAgi,2; |  |
| 4779 | Agi3 J | EQP_HELM | bonus bAgi,3; |  |
| 4780 | Dex1 J | EQP_HELM | bonus bDex,1; |  |
| 4781 | Dex2 J | EQP_HELM | bonus bDex,2; |  |
| 4782 | Dex3 J | EQP_HELM | bonus bDex,3; |  |
| 4783 | Luk1 J | EQP_HELM | bonus bLuk,1; |  |
| 4784 | Luk2 J | EQP_HELM | bonus bLuk,2; |  |
| 4785 | Luk3 J | EQP_HELM | bonus bLuk,3; |  |
| 4786 | Mdef2 | EQP_HELM | bonus bMdef,2; |  |
| 4787 | Mdef4 | EQP_HELM | bonus bMdef,4; |  |
| 4788 | Mdef6 | EQP_HELM | bonus bMdef,6; |  |
| 4789 | Mdef8 | EQP_HELM | bonus bMdef,8; |  |
| 4790 | Mdef10 | EQP_HELM | bonus bMdef,10; |  |
| 4791 | Def3 | EQP_HELM | bonus bDef,3; |  |
| 4792 | Def6 | EQP_HELM | bonus bDef,6; |  |
| 4793 | Def9 | EQP_HELM | bonus bDef,9; |  |
| 4794 | Def12 | EQP_HELM | bonus bDef,12; |  |
| 4795 | HP100 | EQP_HELM | bonus bMaxHP,100; |  |
| 4796 | HP200 | EQP_HELM | bonus bMaxHP,200; |  |
| 4797 | HP300 | EQP_HELM | bonus bMaxHP,300; |  |
| 4798 | HP400 | EQP_HELM | bonus bMaxHP,400; |  |
| 4799 | HP500 | EQP_HELM | bonus bMaxHP,500; |  |
| 4800 | SP50 | EQP_HELM | bonus bMaxSP,50; |  |
| 4801 | SP100 | EQP_HELM | bonus bMaxSP,100; |  |
| 4802 | SP150 | EQP_HELM | bonus bMaxSP,150; |  |
| 4803 | Highness Heal 3sec | EQP_HELM | bonus2 bSkillCooldown,AB_HIGHNESSHEAL,-3000; |  |
| 4804 | Coluceo Heal30 | EQP_HELM | bonus2 bSkillUseSP,AB_CHEAL,30; |  |
| 4805 | Heal Amount2 | EQP_HELM | bonus bHealPower,3; |  |
| 4806 | Matk3 | EQP_HELM | bonus bMatkRate,3; bonus bFixedCastrate,-1; |  |
| 4807 | Atk Speed1 | EQP_HELM | bonus bAspd,1; |  |
| 4808 | Fighting Spirit4 | EQP_HELM | bonus bBaseAtk,15; bonus bHit,5; |  |
| 4809 | Fighting Spirit3 | EQP_HELM | bonus bBaseAtk,12; bonus bHit,4; |  |
| 4810 | Fighting Spirit2 | EQP_HELM | bonus bBaseAtk,9; bonus bHit,3; |  |
| 4811 | Fighting Spirit1 | EQP_HELM | bonus bBaseAtk,6; bonus bHit,2; |  |
| 4812 | Spell4 | EQP_HELM | bonus bMatk,15; bonus bVariableCastrate,-10; |  |
| 4813 | Spell3 | EQP_HELM | bonus bMatk,12; bonus bVariableCastrate,-8; |  |
| 4814 | Spell2 | EQP_HELM | bonus bMatk,9; bonus bVariableCastrate,-6; |  |
| 4815 | Spell1 | EQP_HELM | bonus bMatk,6; bonus bVariableCastrate,-4; |  |
| 4816 | Sharp3 | EQP_HELM | bonus bCritical,12; bonus bHit,4; |  |
| 4817 | Sharp2 | EQP_HELM | bonus bCritical,9; bonus bHit,3; |  |
| 4818 | Sharp1 | EQP_HELM | bonus bCritical,6; bonus bHit,2; |  |
| 4819 | Atk1 | EQP_HELM | bonus bAtkRate,1; |  |
| 4820 | Fighting Spirit5 | EQP_HELM | bonus bBaseAtk,18; bonus bHit,5; |  |
| 4821 | Fighting Spirit6 | EQP_HELM | bonus bBaseAtk,21; bonus bHit,5; |  |
| 4822 | Fighting Spirit7 | EQP_HELM | bonus bBaseAtk,24; bonus bHit,5; |  |
| 4823 | Fighting Spirit8 | EQP_HELM | bonus bBaseAtk,27; bonus bHit,5; |  |
| 4824 | Fighting Spirit9 | EQP_HELM | bonus bBaseAtk,30; bonus bHit,5; |  |
| 4825 | Fighting Spirit10 | EQP_HELM | bonus bBaseAtk,50; bonus bHit,15; |  |
| 4826 | Spell5 | EQP_HELM | bonus bMatk,18; bonus bVariableCastrate,-10; |  |
| 4827 | Spell6 | EQP_HELM | bonus bMatk,21; bonus bVariableCastrate,-10; |  |
| 4828 | Spell7 | EQP_HELM | bonus bMatk,24; bonus bVariableCastrate,-10; |  |
| 4829 | Spell8 | EQP_HELM | bonus bMatk,27; bonus bVariableCastrate,-10; |  |
| 4830 | Spell9 | EQP_HELM | bonus bMatk,30; bonus bVariableCastrate,-10; |  |
| 4831 | Spell10 | EQP_HELM | bonus bMatk,50; bonus bVariableCastrate,-20; |  |
| 4832 | Expert Archer1 | EQP_HELM | bonus bLongAtkRate,2; |  |
| 4833 | Expert Archer2 | EQP_HELM | bonus bLongAtkRate,4; |  |
| 4834 | Expert Archer3 | EQP_HELM | bonus bLongAtkRate,6; |  |
| 4835 | Expert Archer4 | EQP_HELM | bonus bLongAtkRate,8; |  |
| 4836 | Expert Archer5 | EQP_HELM | bonus bLongAtkRate,10; |  |
| 4837 | Expert Archer6 | EQP_HELM | bonus bLongAtkRate,12; |  |
| 4838 | Expert Archer7 | EQP_HELM | bonus bLongAtkRate,14; |  |
| 4839 | Expert Archer8 | EQP_HELM | bonus bLongAtkRate,16; |  |
| 4840 | Expert Archer9 | EQP_HELM | bonus bLongAtkRate,18; |  |
| 4841 | Expert Archer10 | EQP_HELM | bonus bLongAtkRate,20; bonus bAspd,1; |  |
| 4842 | Atk Speed2 | EQP_HELM | bonus bAspd,2; |  |
| 4843 | Sharp4 | EQP_HELM | bonus bCritical,14; bonus bHit,5; |  |
| 4844 | Sharp5 | EQP_HELM | bonus bCritical,15; bonus bHit,6; |  |
| 4845 | Sea Energy | EQP_HELM |  |  |
| 4846 | 2011Valentin Angel | EQP_HELM | bonus bBaseAtk,10; bonus bMatk,10; |  |
| 4847 | 2011Valentin Devil | EQP_HELM | bonus bBaseAtk,10; bonus bMatk,10; |  |
| 4848 | Immuned1 | EQP_HELM | bonus2 bSubEle,Ele_Neutral,5; |  |
| 4849 | Cranial1 | EQP_HELM | bonus2 bAddRaceTolerance,RC_DemiPlayer,5; |  |
| 4850 | Heal Amount3 | EQP_HELM | bonus bHealPower,6; bonus bUseSPrate,5; |  |
| 4851 | Heal Amount4 | EQP_HELM | bonus bHealPower,12; bonus bUseSPrate,10; |  |
| 4852 | Heal Amount5 | EQP_HELM | bonus bHealPower,20; bonus bUseSPrate,15; |  |
| 4853 | S Str | EQP_HELM |  |  |
| 4854 | S Agi | EQP_HELM |  |  |
| 4855 | S Vital | EQP_HELM |  |  |
| 4856 | S Int | EQP_HELM |  |  |
| 4857 | S Dex | EQP_HELM |  |  |
| 4858 | S Luck | EQP_HELM |  |  |
| 4859 | Evasion1 | EQP_HELM | bonus bFlee,1; |  |
| 4860 | Evasion3 | EQP_HELM | bonus bFlee,3; |  |
| 4861 | MHP1 | EQP_HELM | bonus bMaxHPrate,1; |  |
| 4862 | MHP2 | EQP_HELM | bonus bMaxHPrate,2; |  |
| 4863 | Fatal1 | EQP_HELM | bonus bCritAtkRate, 4; bonus bCritical, 1; |  |
| 4864 | Fatal2 | EQP_HELM | bonus bCritAtkRate, 6; bonus bCritical, 2; |  |
| 4865 | Fatal3 | EQP_HELM | bonus bCritAtkRate, 8; bonus bCritical, 3; |  |
| 4866 | Fatal4 | EQP_HELM | bonus bCritAtkRate, 10; bonus bCritical, 4; |  |
| 4867 | MHP3 | EQP_HELM | bonus bMaxHPrate,3; |  |
| 4868 | MHP4 | EQP_HELM | bonus bMaxHPrate,4; |  |
| 4869 | Attack Delay 1 | EQP_HELM | bonus bAspdRate, 4; |  |
| 4870 | SP25 | EQP_HELM | bonus bMaxSP,25; |  |
| 4871 | SP75 | EQP_HELM | bonus bMaxSP,75; |  |
| 4872 | Attack Delay 2 | EQP_HELM | bonus bAspdRate, 6; |  |
| 4873 | Attack Delay 3 | EQP_HELM | bonus bAspdRate, 8; |  |
| 4875 | Bears Power | EQP_HELM |  |  |
| 4876 | Runaway Magic | EQP_HELM |  |  |
| 4877 | Speed Of Light | EQP_HELM |  |  |
| 4878 | Muscle Fool | EQP_HELM |  |  |
| 4879 | Hawkeye | EQP_HELM |  |  |
| 4880 | Lucky Day | EQP_HELM |  |  |
| 4881 | Attack Delay 4 | EQP_HELM | bonus bAspdRate, 10; |  |
| 4882 | Atk1p | EQP_HELM | bonus bAtkRate, 1; |  |
| 4883 | Matk1p | EQP_HELM | bonus bMatkRate, 1; |  |
| 4884 | HIT1 | EQP_HELM | bonus bHit, 1; |  |
| 4885 | Conjure1 | EQP_HELM | bonus bMatk, 5; bonus bVariableCastrate, -3; |  |
| 4886 | Conjure2 | EQP_HELM | bonus bMatk, 10; bonus bVariableCastrate, -3; |  |
| 4887 | Conjure3 | EQP_HELM | bonus bMatk, 15; bonus bVariableCastrate, -3; |  |
| 4888 | Conjure4 | EQP_HELM | bonus bMatk, 20; bonus bVariableCastrate, -3; |  |
| 4889 | Conjure5 | EQP_HELM | bonus bMatk, 30; bonus bVariableCastrate, -5; |  |
| 4890 | Mdef1 | EQP_HELM | bonus bMdef, 1; |  |
| 4891 | Mdef3 | EQP_HELM | bonus bMdef, 3; |  |
| 4892 | Mdef5 | EQP_HELM | bonus bMdef, 5; |  |
| 4893 | Def15 | EQP_HELM | bonus bDef, 15; |  |
| 4894 | Atk4p | EQP_HELM | bonus bAtkRate, 4; |  |
| 4895 | Atk5p | EQP_HELM | bonus bAtkRate, 5; |  |
| 4896 | Matk2p | EQP_HELM | bonus bMatkRate, 2; |  |
| 4897 | Matk3p | EQP_HELM | bonus bMatkRate, 3; |  |
| 4898 | Matk4p | EQP_HELM | bonus bMatkRate, 4; |  |
| 4899 | Matk5p | EQP_HELM | bonus bMatkRate, 5; |  |
| 4900 | MHP5 | EQP_HELM | bonus bMaxHPrate, 5; |  |
| 4902 | Def18 | EQP_HELM | bonus bDef, 18; |  |
| 4903 | Def21 | EQP_HELM | bonus bDef, 21; |  |
| 4904 | Atk6p | EQP_HELM | bonus bAtkRate, 6; |  |
| 4905 | Atk7p | EQP_HELM | bonus bAtkRate, 7; |  |
| 4906 | Matk6p | EQP_HELM | bonus bMatkRate, 6; |  |
| 4907 | Matk7p | EQP_HELM | bonus bAtkRate, 7; |  |
| 4908 | Force1 | EQP_HELM | bonus bStr, 1; bonus bBaseAtk, 3; bonus bInt, -1; |  |
| 4909 | Force2 | EQP_HELM | bonus bStr, 2; bonus bBaseAtk, 6; bonus bInt, -2; |  |
| 4910 | Force3 | EQP_HELM | bonus bStr, 4; bonus bBaseAtk, 12; bonus bInt, -4; |  |
| 4911 | Intellect1 | EQP_HELM | bonus bInt, 1; bonus bMatk, 3; bonus bStr, -1; |  |
| 4912 | Intellect2 | EQP_HELM | bonus bInt, 2; bonus bMatk, 6; bonus bStr, -2; |  |
| 4913 | Intellect3 | EQP_HELM | bonus bInt, 4; bonus bMatk, 12; bonus bStr, -4; |  |
| 4914 | Swiftness1 | EQP_HELM | bonus bAgi, 1; bonus bFlee, 2; bonus bVit, -1; |  |
| 4915 | Swiftness2 | EQP_HELM | bonus bAgi, 2; bonus bFlee, 4; bonus bVit, -2; |  |
| 4916 | Swiftness3 | EQP_HELM | bonus bAgi, 4; bonus bFlee, 8; bonus bVit, -4; |  |
| 4917 | Tough1 | EQP_HELM | bonus bVit, 1; bonus bDef, 3; bonus bMdef, 2; bonus bAgi, -1; |  |
| 4918 | Tough2 | EQP_HELM | bonus bVit, 2; bonus bDef, 6; bonus bMdef, 4; bonus bAgi, -2; |  |
| 4919 | Tough3 | EQP_HELM | bonus bVit, 4; bonus bDef, 12; bonus bMdef, 8; bonus bAgi, -4; |  |
| 4920 | Artful1 | EQP_HELM | bonus bDex, 1; bonus bHit, 2; bonus bLuk, -1; |  |
| 4921 | Artful2 | EQP_HELM | bonus bDex, 2; bonus bHit, 4; bonus bLuk, -2; |  |
| 4922 | Artful3 | EQP_HELM | bonus bDex, 4; bonus bHit, 8; bonus bLuk, -4; |  |
| 4923 | Fortune1 | EQP_HELM | bonus bLuk, 1; bonus bCritical, 1; bonus bDex, -1; |  |
| 4924 | Fortune2 | EQP_HELM | bonus bLuk, 2; bonus bCritical, 2; bonus bDex, -2; |  |
| 4925 | Fortune3 | EQP_HELM | bonus bLuk, 4; bonus bCritical, 4; bonus bDex, -4; |  |
| 4926 | Critical1 | EQP_HELM | bonus bCritical, 1; |  |
| 4927 | HP50 | EQP_HELM | bonus bMaxHP, 50; |  |
| 4928 | SP10 | EQP_HELM | bonus bMaxSP, 10; |  |
| 4929 | MSP1 | EQP_HELM | bonus bMaxSPrate, 1; |  |
| 4930 | HEAL2 | EQP_HELM | bonus bHPrecovRate, 2; |  |
| 4931 | HEALHP1 | EQP_HELM | bonus2 bHPRegenRate, 10, 10000; |  |
| 4932 | HEALSP1 | EQP_HELM | bonus bSPGainValue, 1; |  |
| 4933 | Tolerance Not1 | EQP_HELM | bonus2 bSubEle, Ele_Neutral, 1; |  |
| 4934 | Tolerance Not2 | EQP_HELM | bonus2 bSubEle, Ele_Neutral, 2; |  |
| 4935 | Tolerance Not3 | EQP_HELM | bonus2 bSubEle, Ele_Neutral, 3; |  |
| 4936 | ATK BIG1 | EQP_HELM | bonus2 bAddSize, Size_Large, 1; |  |
| 4937 | ATK MEDIUM1 | EQP_HELM | bonus2 bAddSize, Size_Medium, 1; |  |
| 4938 | ATK SMALL1 | EQP_HELM | bonus2 bAddSize, Size_Small, 1; |  |
| 4939 | Critical2 | EQP_HELM | bonus bCritical, 2; |  |
| 4940 | Critical3 | EQP_HELM | bonus bCritical, 4; |  |
| 4941 | Critical4 | EQP_HELM | bonus bCritical, 6; |  |
| 4942 | Dodge1 | EQP_HELM | bonus bCritical, 3; |  |
| 4943 | Dodge2 | EQP_HELM | bonus bFlee2, 4; |  |
| 4944 | Dodge3 | EQP_HELM | bonus bFlee2, 5; |  |
| 4945 | Thrift1 | EQP_HELM | bonus bUseSPrate, -2; |  |
| 4946 | Thrift2 | EQP_HELM | bonus bUseSPrate, -4; |  |
| 4947 | Thrift3 | EQP_HELM | bonus bUseSPrate, -6; |  |
| 4948 | Skill Delay1 | EQP_HELM | bonus bDelayrate, -2; |  |
| 4949 | Skill Delay2 | EQP_HELM | bonus bDelayrate, -4; |  |
| 4950 | Skill Delay3 | EQP_HELM | bonus bDelayrate, -6; |  |
| 4951 | Darkness Drop | EQP_HELM | bonus3 bAddEle, Ele_Dark, 2, BF_WEAPON - BF_MAGIC; bonus2 bSubEle, Ele_Dark, 1; |  |
| 4952 | Fire Drop | EQP_HELM | bonus3 bAddEle, Ele_Fire, 2, BF_WEAPON - BF_MAGIC; bonus2 bSubEle, Ele_Fire, 1; |  |
| 4953 | Water Drop | EQP_HELM | bonus3 bAddEle, Ele_Water, 2, BF_WEAPON - BF_MAGIC; bonus2 bSubEle, Ele_Water, 1; |  |
| 4954 | Earth Drop | EQP_HELM | bonus3 bAddEle, Ele_Earth, 2, BF_WEAPON - BF_MAGIC; bonus2 bSubEle, Ele_Earth, 1; |  |
| 4955 | Light Drop | EQP_HELM | bonus3 bAddEle, Ele_Holy, 2, BF_WEAPON - BF_MAGIC; bonus2 bSubEle, Ele_Holy, 1; |  |
| 4956 | Recovery Drop | EQP_HELM | bonus2 bSkillHeal, AL_HEAL, 2; bonus2 bSkillHeal, PR_SANCTUARY, 2; bonus2 bSkillHeal, AM_POTIONPITCHER, 2; bonus2 bSkill |  |
| 4957 | The Power Of Famitsu | EQP_HELM | bonus bMaxHP, 832; |  |
| 4958 | Double Pediatric Palace | EQP_HELM | bonus bDelayrate, -1; |  |
| 4959 | Sagittarius | EQP_HELM | bonus bLongAtkRate, 1; |  |
| 4960 | Aquarius | EQP_HELM | bonus bUseSPrate, -2; |  |
| 4961 | Aries | EQP_HELM | bonus bMatk, 10; |  |
| 4962 | Cancer | EQP_HELM | bonus bBaseAtk, 3; |  |
| 4963 | Taurus | EQP_HELM | bonus bBaseAtk, 3; bonus bHit, 1; |  |
| 4964 | Capricorn | EQP_HELM | bonus bMatkRate, 3; |  |
| 4965 | Pisces | EQP_HELM | bonus bVariableCastrate, -2; |  |
| 4966 | Scorpio | EQP_HELM |  |  |
| 4967 | Leo | EQP_HELM | bonus bAtkRate, 3; |  |
| 4968 | Virgo | EQP_HELM | bonus2 bSkillHeal, AL_HEAL, 1; bonus2 bSkillHeal, PR_SANCTUARY, 1; bonus2 bSkillHeal, AM_POTIONPITCHER, 1; bonus2 bSkill |  |
| 4969 | Libra | EQP_HELM | bonus2 bSkillHeal2, AL_HEAL, 1; bonus2 bSkillHeal2, PR_SANCTUARY, 1; bonus2 bSkillHeal2, AM_POTIONPITCHER, 1; bonus2 bSk |  |
| 4970 | Fire Property Reactor | EQP_HELM | bonus bDefEle, Ele_Fire; |  |
| 4971 | Water Property Reactor | EQP_HELM | bonus bDefEle, Ele_Water; |  |
| 4972 | Earth Property Reactor | EQP_HELM | bonus bDefEle, Ele_Earth; |  |
| 4973 | Wind Property Reactor | EQP_HELM | bonus bDefEle, Ele_Wind; |  |
| 4974 | Fire Resistance Reactor | EQP_HELM | bonus2 bSubEle, Ele_Fire, 25; bonus2 bSubEle, Ele_Water, -25; |  |
| 4975 | Water Resistance Reactor | EQP_HELM | bonus2 bSubEle, Ele_Water, 25; bonus2 bSubEle, Ele_Wind, -25; |  |
| 4976 | Earth Resistance Reactor | EQP_HELM | bonus2 bSubEle, Ele_Earth, 25; bonus2 bSubEle, Ele_Fire, -25; |  |
| 4977 | Wind Resistance Reactor | EQP_HELM | bonus2 bSubEle, Ele_Wind, 25; bonus2 bSubEle, Ele_Earth, -25; |  |
| 4978 | Restoration Reactor 101 | EQP_HELM | bonus2 bHPRegenRate, (getrefine() >= 7) ? 100 : 50, 5000; |  |
| 4979 | Restoration Reactor 102 | EQP_HELM | bonus2 bSPRegenRate, (getrefine() >= 7) ? 5 : 3, 500; |  |
| 4980 | Restoration Reactor 201 | EQP_HELM | bonus bHPrecovRate, (getrefine() >= 7) ? 100 : 50; |  |
| 4981 | Restoration Reactor 202 | EQP_HELM | bonus bSPrecovRate, (getrefine() >= 7) ? 100 : 50; |  |
| 4982 | Auxiliary Reactor STR | EQP_HELM | if (readparam(bStr) >= 10) bonus bBaseAtk, 5; if (getrefine() >= 7) bonus bBaseAtk, 10; |  |
| 4983 | Auxiliary Reactor INT | EQP_HELM | if (readparam(bInt) >= 10) bonus bMatk, 5; if (getrefine() >= 7) bonus bMatk, 10; |  |
| 4984 | Auxiliary Reactor DEF | EQP_HELM | bonus bDef, 100; |  |
| 4985 | Auxiliary Reactor Perfect | EQP_HELM | bonus bFlee2, 3; |  |
| 4986 | Auxiliary Reactor Attack | EQP_HELM | bonus bBaseAtk, 20; |  |
| 4987 | Auxiliary Reactor Magic | EQP_HELM | bonus bMatk, 20; |  |
| 4988 | Auxiliary Reactor MaxHP | EQP_HELM | bonus bMaxHPrate, 5; |  |
| 4989 | Auxiliary Reactor MaxSP | EQP_HELM | bonus bMaxSPrate, 3; |  |
| 4990 | Auxiliary Reactor Frozen | EQP_HELM | bonus2 bResEff, Eff_Freeze, 10000; |  |
| 4991 | Auxiliary Reactor ASPD | EQP_HELM | bonus bAspd, 1; |  |
| 4992 | HPdrain1 | EQP_HELM | bonus2 bHPDrainRate, 1, 1; |  |
| 4993 | SPdrain1 | EQP_HELM | bonus2 bSPDrainRate, 1, 1; |  |
| 4994 | Rune Of Might1 | EQP_HELM | if (getrefine() >= 7) bonus bStr, 5; if (getrefine() >= 10) bonus bAtkRate, 10; |  |
| 4995 | Rune Of Might2 | EQP_HELM |  |  |
| 4996 | Rune Of Might3 | EQP_HELM |  |  |
| 4997 | Rune Of Agility1 | EQP_HELM | if (getrefine() >= 7) bonus bAgi, 5; if (getrefine() >= 10) bonus bFlee2, 5; |  |
| 4998 | Rune Of Agility2 | EQP_HELM |  |  |
| 4999 | Rune Of Agility3 | EQP_HELM |  |  |
| 27164 | Faceworm Queen Card | 64 | bonus(bMaxHPrate, -10); bonus(bCritical, 15 + getrefine()); bonus(bCritAtkRate, getrefine()); | FACEWORM_QUEEN:1 |
| 27182 | Captain Felock Card | EQP_WEAPON | bonus(bAtk, 30); bonus2(bSkillAtk, RL_AM_BLAST, getrefine() >= 10 ? 60 : 30); bonus2(bSkillAtk, RL_HAMMER_OF_GOD, getref | E1_FELOCK:1 |
| 27352 | Rigid Nightmare Terror Card | EQP_SHOES | bonus(bMaxSPrate, 5); | NIGHTMARE_TERROR_H:10 |
| 27361 | Contaminated Wanderer Card | EQP_WEAPON | bonus2(bAddSize,Size_Medium, 30); bonus2(bAddSize,Size_Large, 30); | WANDER_MAN_H:10 |
| 29000 | Rune Of Magic1 | EQP_HELM | if (getrefine() >= 7) bonus bInt, 5; if (getrefine() >= 10) bonus bMatkRate, 5; |  |
| 29001 | Rune Of Magic2 | EQP_HELM |  |  |
| 29002 | Rune Of Magic3 | EQP_HELM |  |  |
| 29003 | Rune Of Dexterity1 | EQP_HELM | if (getrefine() >= 7) bonus bDex, 5; if (getrefine() >= 10) bonus bLongAtkRate, 5; |  |
| 29004 | Rune Of Dexterity2 | EQP_HELM |  |  |
| 29005 | Rune Of Dexterity3 | EQP_HELM |  |  |
| 29006 | Rune Of Fortune1 | EQP_HELM | if (getrefine() >= 7) bonus bLuk, 5; if (getrefine() >= 10) bonus bCritAtkRate, 5; |  |
| 29007 | Rune Of Fortune2 | EQP_HELM |  |  |
| 29008 | Rune Of Fortune3 | EQP_HELM |  |  |
| 29009 | Rune Of Stamina1 | EQP_HELM | if (getrefine() >= 7) bonus bVit, 5; if (getrefine() >= 10) bonus bMaxHPrate, 5; |  |
| 29010 | Rune Of Stamina2 | EQP_HELM |  |  |
| 29011 | Rune Of Stamina3 | EQP_HELM |  |  |
| 29013 | HPAbsorb3 | EQP_HELM | bonus2 bHPDrainRate, 1, 3; |  |
| 29014 | STR3 INT3 | EQP_HELM | bonus bStr, 3; bonus bInt, -3; |  |
| 29015 | STR3 DEX3 | EQP_HELM | bonus bStr, 3; bonus bDex, -3; |  |
| 29016 | INT3 DEX3 | EQP_HELM | bonus bInt, 3; bonus bDex, -3; |  |
| 29017 | INT3 VIT3 | EQP_HELM | bonus bInt, 3; bonus bVit, -3; |  |
| 29018 | DEX3 VIT3 | EQP_HELM | bonus bDex, 3; bonus bVit, -3; |  |
| 29019 | DEX3 AGI3 | EQP_HELM | bonus bDex, 3; bonus bAgi, -3; |  |
| 29020 | VIT3 AGI3 | EQP_HELM | bonus bVit, 3; bonus bAgi, -3; |  |
| 29021 | VIT3 LUK3 | EQP_HELM | bonus bVit, 3; bonus bLuk, -3; |  |
| 29022 | AGI3 LUK3 | EQP_HELM | bonus bAgi, 3; bonus bLuk, -3; |  |
| 29023 | AGI3 STR3 | EQP_HELM | bonus bAgi, 3; bonus bStr, -3; |  |
| 29024 | LUK3 STR3 | EQP_HELM | bonus bLuk, 3; bonus bStr, -3; |  |
| 29025 | LUK3 INT3 | EQP_HELM | bonus bLuk, 3; bonus bInt, -3; |  |
| 29026 | DEF20 | EQP_HELM | bonus bDef, 20; |  |
| 29027 | EXP2 | EQP_HELM | bonus2 bExpAddRace, RC_All, 2; |  |
| 31022 | Abandoned Teddy Bear Card | EQP_SHOES | bonus(bMaxSPrate, 20); bonus2(bAddEff2, Eff_Curse, 20); | XM_TEDDY_BEAR:1 |


**Total: 1012 cards**

See `cards.json` for raw structured data (all drops, full scripts).