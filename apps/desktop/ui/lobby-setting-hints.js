// Presentation only. Adapted from legacy nt-app/src/store/index.js flagInfo/defaultFlags.
// No defaults, permissions or gameplay decisions are derived from these hints.
export const LOBBY_SETTING_HINTS=Object.freeze({
  publicStats:{label:"Public stats",help:"Allow anyone to view player names and gameplay statistics for this lobby. Viewers cannot join or control the game.",timing:"live"},
  "seedPolicy": {
    "label": "Worlds",
    "help": "Choose a shared random/fixed seed or separate random worlds. A new world and explicit host Start are required.",
    "timing": "next-run"
  },
  "seed": {
    "label": "Shared seed",
    "help": "All players use this shared seed. Zero with shared-random selects one seed when Create or Save succeeds.",
    "timing": "next-run"
  },
  "generateSharedSeed": {
    "label": "Generate new random seed",
    "help": "Generates a nonzero uint32 in your browser, selects Shared fixed seed and fills the visible seed draft. Create or Save commits it; this button does not change a world immediately.",
    "timing": "draft"
  },
  "randomizeLoot": {
    "label": "Randomize loot",
    "help": "On a shared seed, let each player find different loot. Unavailable for separate random worlds.",
    "timing": "next-run"
  },
  "mode": {
    "label": "Mode",
    "help": "Choose a game mode supported by this service. Changing mode requires a new run.",
    "timing": "next-run"
  },
  "testingMode": {
    "label": "Testing mode",
    "help": "Dev-only starting perk and wand pickups in a new protected world, when this service supports them.",
    "timing": "next-run"
  },
  "NT_sync_hearts": {
    "label": "Share hearts",
    "help": "When someone picks up a heart, the other eligible players receive its shared effect.",
    "timing": "live"
  },
  "NT_sync_orbs": {
    "label": "Share orbs",
    "help": "When someone picks up an orb, other eligible players receive it, subject to their orb limits and ownership.",
    "timing": "live"
  },
  "NT_sync_shift": {
    "label": "Share fungal shifts",
    "help": "Share the same fungal conversion with other players. Their local cooldown and shift limit still apply.",
    "timing": "live"
  },
  "NT_team_perks": {
    "label": "Team perks",
    "help": "Share the legacy team subset of perks. Share all perks takes precedence when both are enabled.",
    "timing": "live"
  },
  "NT_sync_perks": {
    "label": "Share all perks",
    "help": "When someone takes a supported perk, the other eligible players receive it too. Takes precedence over Team perks.",
    "timing": "live"
  },
  "NT_alt_heartache": {
    "label": "Change Heartache",
    "help": "Make Heartache affect current HP instead of maximum HP. An effect already started keeps its captured choice.",
    "timing": "live"
  },
  "NT_ban_perks": {
    "label": "Ban Glass Cannon",
    "help": "Remove Glass Cannon from the perk pool and held perks; Gamble is filtered too. Recommended for large Respawn Penalty lobbies.",
    "timing": "live"
  },
  "NT_sync_orbs_no_curse": {
    "label": "Friendly parallel-world orbs",
    "help": "Remote parallel-world orbs give other players a normal heart. The collector keeps the original cursed heart. Requires Share orbs.",
    "timing": "live"
  },
  "NT_sync_orb_count": {
    "label": "Shared orb receiving limit",
    "help": "Stop receiving shared orbs at this count. Zero means unlimited; local orb collection is unaffected.",
    "timing": "live"
  },
  "NT_sync_steve": {
    "label": "Share angering the gods",
    "help": "Anger the gods for everyone on subsequent shared events.",
    "timing": "live"
  },
  "NT_send_gold": {
    "label": "Send Gold",
    "help": "Allow players to deposit and withdraw shared gold in the next run.",
    "timing": "next-run"
  },
  "NT_send_wands": {
    "label": "Send Wands",
    "help": "Allow players to deposit and take wands in the next run.",
    "timing": "next-run"
  },
  "NT_send_flasks": {
    "label": "Send Flasks",
    "help": "Allow players to deposit and take flasks in the next run.",
    "timing": "next-run"
  },
  "NT_send_items": {
    "label": "Send Items",
    "help": "Allow players to deposit and take supported items in the next run.",
    "timing": "next-run"
  },
  "NT_death_penalty": {
    "label": "On death",
    "help": "End run finishes the shared run for all participants. Respawn Penalty removes 20% maximum HP; a terminal low-life death ends the shared run without another penalty cascade. Respawn returns the player to the last checkpoint without an HP penalty. Wins are separate. An accepted death keeps its captured rules.",
    "timing": "live"
  },
  "NT_ondeath_kick": {
    "label": "Kick on death",
    "help": "Remove the reporting player, including the host, from participation in this run. The host keeps lobby controls. Same-run Join or Continue cannot undo removal; wins are exempt. The selected death mode still applies.",
    "timing": "live"
  },
  "name": {
    "label": "Lobby name",
    "help": "User accounts use their generated lobby name. Power User and higher may choose a custom name.",
    "timing": "admission"
  },
  "capacity": {
    "label": "Capacity",
    "help": "Maximum admitted players, within your current account entitlement. Existing players must fit the new limit.",
    "timing": "admission"
  },
  "visibility": {
    "label": "Privacy",
    "help": "Public lobbies appear in discovery. Private lobbies are visible through existing membership or invitations.",
    "timing": "admission"
  },
  "password": {
    "label": "Password",
    "help": "Require the password for a new player to join. Existing players keep their membership. Leave an existing lobby password blank to keep it.",
    "timing": "admission"
  },
  "clearPassword": {
    "label": "Remove password",
    "help": "Remove the existing lobby password when admission settings are saved.",
    "timing": "admission"
  },
  "allowGuests": {
    "label": "Allow guests",
    "help": "Permit new guest joins where supported by the service.",
    "timing": "admission"
  },
  "joinLocked": {
    "label": "Lock new joins",
    "help": "Block new non-host joins. Current members and the host may reconnect; this does not kick or mute anyone.",
    "timing": "admission"
  },
  "accounts": {
    "label": "Invitations",
    "help": "Saving replaces the complete invited-account list. Review the full list before saving; current members keep access.",
    "timing": "admission"
  }
});
