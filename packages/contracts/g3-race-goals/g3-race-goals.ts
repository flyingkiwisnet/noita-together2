// GENERATED from schema.json and catalogue.json. Do not edit. No transport contract.
export const RACE_GOAL_SCHEMA = {
  "revision": "race-goals-1",
  "scope": "Pure catalogue types; no Race RPC, admission or persistence contract yet.",
  "limits": {
    "goals": 16,
    "namedBosses": 16,
    "entrants": 120,
    "parallelIndex": 10,
    "hpObservation": 2147483647,
    "goldObservation": 2147483647,
    "elapsedMs": 9007199254740991,
    "durationMs": 604800000
  },
  "goals": [
    {
      "id": "classic_ending",
      "category": "ending",
      "parameters": {},
      "presets": [],
      "difficulty": "hard",
      "randomEligible": false,
      "description": "Complete the traditional underground Gold/Greed ending after the other goals.",
      "value": "sticky boolean; other endings do not count"
    },
    {
      "id": "collect_orbs",
      "category": "exploration",
      "parameters": {
        "target": {
          "type": "integer",
          "min": 1,
          "max": 33
        }
      },
      "presets": [
        1,
        3,
        5,
        7,
        11,
        15,
        20,
        33
      ],
      "difficultyBands": [
        [
          3,
          "easy"
        ],
        [
          7,
          "medium"
        ],
        [
          11,
          "hard"
        ],
        [
          33,
          "extreme"
        ]
      ],
      "randomMax": 11,
      "description": "Collect distinct Orbs of True Knowledge in the admitted race world.",
      "value": "distinct canonical orb IDs, including verified world offsets; never a client-supplied total"
    },
    {
      "id": "defeat_bosses",
      "category": "combat",
      "parameters": {
        "target": {
          "type": "integer",
          "min": 1,
          "max": 16
        }
      },
      "presets": [
        1,
        2,
        3,
        5
      ],
      "difficultyBands": [
        [
          1,
          "medium"
        ],
        [
          3,
          "hard"
        ],
        [
          16,
          "extreme"
        ]
      ],
      "randomMax": 3,
      "description": "Confirm final defeats of distinct supported boss types.",
      "value": "distinct boss-type IDs; phases and Parallel World repeats never add another type"
    },
    {
      "id": "defeat_named_bosses",
      "category": "combat",
      "parameters": {
        "bossIds": {
          "type": "ids",
          "registry": "bosses",
          "max": 16
        }
      },
      "presets": [],
      "description": "Defeat every selected boss; other bosses cannot substitute.",
      "value": "all-of selected IDs; each first accepted defeat time is retained"
    },
    {
      "id": "visit_biome",
      "category": "exploration",
      "parameters": {
        "biomeId": {
          "type": "id",
          "registry": "biomes"
        }
      },
      "presets": [],
      "description": "Enter the selected verified biome with the player in any world.",
      "value": "sticky visit; camera location and merely adjacent regions do not count"
    },
    {
      "id": "visit_parallel_world",
      "category": "exploration",
      "parameters": {
        "direction": {
          "type": "enum",
          "values": [
            "east",
            "west"
          ]
        },
        "index": {
          "type": "integer",
          "min": 1,
          "max": 10
        }
      },
      "presets": [
        1,
        2,
        3
      ],
      "difficultyBands": [
        [
          1,
          "hard"
        ],
        [
          10,
          "extreme"
        ]
      ],
      "randomMax": 1,
      "description": "Enter the selected direction and numbered Parallel World region.",
      "value": "furthest verified entered region separately east/west; higher same-direction index satisfies lower"
    },
    {
      "id": "reach_max_hp",
      "category": "resources",
      "parameters": {
        "target": {
          "type": "integer",
          "min": 500,
          "max": 25000
        }
      },
      "presets": [
        500,
        1000,
        2000,
        5000,
        10000,
        25000
      ],
      "difficultyBands": [
        [
          500,
          "easy"
        ],
        [
          2000,
          "medium"
        ],
        [
          10000,
          "hard"
        ],
        [
          25000,
          "extreme"
        ]
      ],
      "randomMax": 10000,
      "description": "Reach the threshold of maximum HP, in displayed HP units.",
      "value": "high-water maximum HP; current HP damage is irrelevant; completion survives max-HP loss"
    },
    {
      "id": "accumulate_gold",
      "category": "resources",
      "parameters": {
        "target": {
          "type": "integer",
          "min": 10000,
          "max": 1000000
        }
      },
      "presets": [
        10000,
        25000,
        50000,
        100000,
        250000,
        1000000
      ],
      "difficultyBands": [
        [
          10000,
          "easy"
        ],
        [
          50000,
          "medium"
        ],
        [
          250000,
          "hard"
        ],
        [
          1000000,
          "extreme"
        ]
      ],
      "randomMax": 250000,
      "description": "Hold the required wallet balance at one time.",
      "value": "wallet high-water; spending does not undo completion; deposits/withdrawals and lifetime sums are not observations"
    }
  ],
  "observations": [
    {
      "kind": "orb",
      "fields": {
        "orbId": "number"
      }
    },
    {
      "kind": "boss",
      "fields": {
        "bossId": "string"
      }
    },
    {
      "kind": "biome",
      "fields": {
        "biomeId": "string"
      }
    },
    {
      "kind": "parallel",
      "fields": {
        "direction": "'east' | 'west'",
        "index": "number"
      }
    },
    {
      "kind": "max_hp",
      "fields": {
        "value": "number"
      }
    },
    {
      "kind": "gold",
      "fields": {
        "value": "number"
      }
    },
    {
      "kind": "ending",
      "fields": {
        "ending": "'classic' | 'other'"
      }
    }
  ]
} as const;
export const RACE_GOAL_CATALOGUE = {
  "revision": "race-goals-1",
  "evidence": "docs/g3/race-goal-detectors.json",
  "supportMeaning": "Source-verified detector design only; no production Race integration or actual-engine acceptance is implied.",
  "goalSupport": {
    "classic_ending": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending. Only exact underground sequence, not generic game-completed; requires fewer than33 orbs. Exactly11 underground omits gold_effect and is excluded too; only normal gold_effect branch counts."
    },
    "collect_orbs": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending. Ordinary IDs0..10 with world offsets; rare ID11 can be observed but is not a promised route. Precise physical33-orb availability remains uncertain; no NG+ continuity."
    },
    "defeat_bosses": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending."
    },
    "defeat_named_bosses": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending."
    },
    "visit_biome": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending."
    },
    "visit_parallel_world": {
      "available": true,
      "reason": "Client source-backed default-map conservative design: actual engine PW index plus inferred wrapped safe-interior admission; this can delay legitimate edge-scene credit and is not an exact engine boundary. Engine checks and adapter pending."
    },
    "reach_max_hp": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending."
    },
    "accumulate_gold": {
      "available": true,
      "reason": "Source-verified by Client local game/current/legacy inspection; detector adapter and genuine engine smoke still pending."
    }
  },
  "orbs": [
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    128,
    129,
    130,
    131,
    132,
    133,
    134,
    135,
    136,
    137,
    138,
    139,
    256,
    257,
    258,
    259,
    260,
    261,
    262,
    263,
    264,
    265,
    266,
    267
  ],
  "bosses": [
    {
      "id": "kolmisilma",
      "name": "Kolmisilmä",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Reach the final underground arena and Sampo encounter."
      ],
      "detector": "data/entities/animals/boss_centipede/death_check.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_centipede/boss_centipede.xml"
      ],
      "deathScript": "data/entities/animals/boss_centipede/death_check.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/0"
    },
    {
      "id": "high_alchemist",
      "name": "High Alchemist",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Reach the Abandoned Alchemy Laboratory encounter ($biome_secret_lab); not Ancient Laboratory ($biome_liquidcave)."
      ],
      "detector": "data/entities/animals/boss_alchemist/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_alchemist/boss_alchemist.xml"
      ],
      "deathScript": "data/entities/animals/boss_alchemist/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/1"
    },
    {
      "id": "wand_connoisseur",
      "name": "Wand Connoisseur",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Reach the pit encounter; confirm root final death."
      ],
      "detector": "data/entities/animals/boss_pit/boss_pit_death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_pit/boss_pit.xml"
      ],
      "deathScript": "data/entities/animals/boss_pit/boss_pit_death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/2"
    },
    {
      "id": "pyramid_boss",
      "name": "Pyramid Boss",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Reach the Pyramid; wait for final root defeat, not a detached limb."
      ],
      "detector": "data/entities/animals/boss_limbs/boss_limbs_death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_limbs/boss_limbs.xml"
      ],
      "deathScript": "data/entities/animals/boss_limbs/boss_limbs_death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/3"
    },
    {
      "id": "dragon",
      "name": "Dragon",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Trigger the dragon encounter; use its dynamically attached death callback."
      ],
      "detector": "data/scripts/animals/boss_dragon_death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_dragon.xml"
      ],
      "deathScript": "data/scripts/animals/boss_dragon_death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/4"
    },
    {
      "id": "leviathan",
      "name": "Leviathan",
      "available": true,
      "difficulty": "extreme",
      "randomEligible": false,
      "aliases": [],
      "prerequisites": [
        "Reach the lake encounter."
      ],
      "detector": "data/entities/animals/boss_fish/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_fish/fish_giga.xml"
      ],
      "deathScript": "data/entities/animals/boss_fish/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/5"
    },
    {
      "id": "master_of_masters",
      "name": "Master of Masters",
      "available": true,
      "difficulty": "extreme",
      "randomEligible": false,
      "aliases": [],
      "prerequisites": [
        "Reach the wizard encounter; confirm final root death."
      ],
      "detector": "data/entities/animals/boss_wizard/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_wizard/boss_wizard.xml"
      ],
      "deathScript": "data/entities/animals/boss_wizard/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/6"
    },
    {
      "id": "mecha_kolmi",
      "name": "Mecha Kolmi",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Reach the robot encounter."
      ],
      "detector": "data/entities/animals/boss_robot/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_robot/boss_robot.xml"
      ],
      "deathScript": "data/entities/animals/boss_robot/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/7"
    },
    {
      "id": "tiny",
      "name": "Tiny",
      "available": true,
      "difficulty": "extreme",
      "randomEligible": false,
      "aliases": [],
      "prerequisites": [
        "Reach the Tiny encounter."
      ],
      "detector": "data/entities/animals/maggot_tiny/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/maggot_tiny/maggot_tiny.xml"
      ],
      "deathScript": "data/entities/animals/maggot_tiny/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/8"
    },
    {
      "id": "meat_boss",
      "name": "Meat Boss",
      "available": true,
      "difficulty": "extreme",
      "randomEligible": false,
      "aliases": [],
      "prerequisites": [
        "Reach the meat boss encounter."
      ],
      "detector": "data/entities/animals/boss_meat/death.lua",
      "supportedContentVersion": "Noita-Jan25-2025",
      "entityFiles": [
        "data/entities/animals/boss_meat/boss_meat.xml"
      ],
      "deathScript": "data/entities/animals/boss_meat/death.lua",
      "variantPolicy": "Only this exact root file; parallel instances use the same type ID and credit once. No filename prefix, translated-name, disappearance or damage-only matching.",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/bosses/9"
    }
  ],
  "biomes": [
    {
      "id": "coal_pits",
      "name": "Coal Pits",
      "gameKey": "$biome_excavationsite",
      "available": true,
      "difficulty": "easy",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_excavationsite",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/0"
    },
    {
      "id": "snowy_depths",
      "name": "Snowy Depths",
      "gameKey": "$biome_snowcave",
      "available": true,
      "difficulty": "easy",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_snowcave",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/1"
    },
    {
      "id": "hiisi_base",
      "name": "Hiisi Base",
      "gameKey": "$biome_snowcastle",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_snowcastle",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/2"
    },
    {
      "id": "underground_jungle",
      "name": "Underground Jungle",
      "gameKey": "$biome_rainforest",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_rainforest",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/3"
    },
    {
      "id": "the_vault",
      "name": "The Vault",
      "gameKey": "$biome_vault",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_vault",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/4"
    },
    {
      "id": "temple_of_the_art",
      "name": "Temple of the Art",
      "gameKey": "$biome_crypt",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_crypt",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/5"
    },
    {
      "id": "fungal_caverns",
      "name": "Fungal Caverns",
      "gameKey": "$biome_fungicave",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_fungicave",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/6"
    },
    {
      "id": "lukki_lair",
      "name": "Lukki Lair",
      "gameKey": "$biome_rainforest_dark",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_rainforest_dark",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/7"
    },
    {
      "id": "ancient_laboratory",
      "name": "Ancient Laboratory",
      "gameKey": "$biome_liquidcave",
      "available": true,
      "difficulty": "hard",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_liquidcave",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/8"
    },
    {
      "id": "pyramid",
      "name": "Pyramid",
      "gameKey": "$biome_pyramid",
      "available": true,
      "difficulty": "medium",
      "randomEligible": true,
      "aliases": [],
      "prerequisites": [
        "Player enters the verified biome interior in any world; camera position does not count."
      ],
      "detector": "BiomeMapGetName(localActorX,localActorY) === $biome_pyramid",
      "supportedContentVersion": "Noita-Jan25-2025",
      "evidenceRef": "docs/g3/race-goal-detectors.json#/biomes/9"
    }
  ],
  "limitations": [
    "One account entry per lobby and same admitted-world recovery are later integration prerequisites.",
    "Maximum orb target remains33. Physical33-orb route availability is uncertain; costly NG+ continuity is deferred, and NG+ is unavailable in this catalogue environment. Ordinary orbs0..10 are distinct across main/west/east; rare chest11 is observable, not a guaranteed random route; unknown13 is excluded.",
    "Parallel visits support only default-map geometry. Require actual engine world index AND conservative wrapped interior [-15872,15872), with width BiomeMapGetSize()*512 (35840 on default map). This inferred interior avoids walls but can delay legitimate edge-scene credit; no universal four-wall-column or exact earliest-boundary proof. Alternate maps and NG+ unavailable.",
    "Classic underground Gold/Greed emits the normal gold_effect only at fewer than33 orbs and not exactly11. Classic plus collect33 is rejected; collect11 can complete first and then end at12..32. Generic completed-game/progress_ending0 flags cannot substitute.",
    "Source-verified candidates are available to catalogue validation only. No production detector, game-launch evidence or complete Race integration is claimed.",
    "Difficulty and random eligibility are provisional. Manual extremes remain selectable; random excludes extreme/terminal goals, near duplicates and overlapping boss objectives."
  ],
  "supportedContentVersion": "Noita-Jan25-2025",
  "supportStatus": "source_verified_engine_pending",
  "unavailableBossCandidates": [
    {
      "candidate": "Four Gate Guardians",
      "status": "unavailable",
      "available": false,
      "reason": "Shared death callback sets per-name killed flags and an all-four persistent unlock. Initial canonical boss-type/group meaning and recovery attribution are not established; do not count four arbitrary phases as one or four bosses."
    },
    {
      "candidate": "Kivi/Lohkare",
      "status": "unavailable",
      "available": false,
      "reason": "death() triggers boss_phase2_marker apparition spawns. Final variant/phase grouping and reliable encounter route not verified; generic first death would overcredit."
    },
    {
      "candidate": "Forgotten",
      "status": "unavailable",
      "available": false,
      "reason": "Ghost spawner explicitly replaces/kills another root; reliable encounter/access and transformation grouping remain unverified. Despawn/replacement is not a defeat."
    },
    {
      "candidate": "Tapion vasalli/island spirit",
      "status": "unavailable",
      "available": false,
      "reason": "Final death callback exists but encounter prerequisites/variant boundaries not investigated enough for initial offering."
    }
  ]
} as const;
export type Difficulty = 'easy' | 'medium' | 'hard' | 'extreme';
export type Goal = {instanceId:string} & (
  | {id: "classic_ending"; parameters: {}}
  | {id: "collect_orbs"; parameters: {target: number}}
  | {id: "defeat_bosses"; parameters: {target: number}}
  | {id: "defeat_named_bosses"; parameters: {bossIds: string[]}}
  | {id: "visit_biome"; parameters: {biomeId: string}}
  | {id: "visit_parallel_world"; parameters: {direction: "east" | "west"; index: number}}
  | {id: "reach_max_hp"; parameters: {target: number}}
  | {id: "accumulate_gold"; parameters: {target: number}}
);
export type Observation =
  | {kind: "orb"; orbId: number}
  | {kind: "boss"; bossId: string}
  | {kind: "biome"; biomeId: string}
  | {kind: "parallel"; direction: 'east' | 'west'; index: number}
  | {kind: "max_hp"; value: number}
  | {kind: "gold"; value: number}
  | {kind: "ending"; ending: 'classic' | 'other'};
