# Deckbound — Scenario & Tutorial Plan (build blueprint)

> **Status: the authored target for the new combat.** This is the blueprint the code and
> `booklet.ron` are being rebuilt to match (spec-first: rules in `canon/2-spec/README.md` §1,
> §3, §4 are authoritative; this doc is the *content* plan that realizes them). Living — update
> as content firms up.

## Combat mode

- **Deterministic base (canonical floor):** no Clash. A **same-range** engagement is a
  **simultaneous trade** (both deal base through armor/toughness, §2); a **range mismatch** is an
  **auto-hit** (§4.2). All of lanes / roles / phases / Tempo / Focus runs on top.
- **Clash module (optional):** when enabled, a same-range engagement instead runs the four-card
  Clash (§1.0, Strike/Anticipate/Gather/Evade + Force). Per-scenario switch. The base tutorials
  run base mode; the Clash-module tutorials turn it on.

## Cast — fresh specialists (roles × attack type)

Numbers are first-pass (AI-seeded, human-tuned). Attack = melee / ranged / both / neither.

| Name      | Role                     | Attack  | Profile (seed)                                   | Signature cards             |
| --------- | ------------------------ | ------- | ------------------------------------------------ | --------------------------- |
| **Anvil** | Wall (Vanguard)          | melee   | Body 10 · armor heavy · Mind 5 (Focus) · Speed 2 | Phalanx, Bodyguard, Taunt   |
| **Wisp**  | Infiltrator (Skirmisher) | melee   | Body 4 · Speed 7 · Power 4                       | Blitz, Shadowstep, Backstab |
| **Sear**  | Artillery (Reserve)      | ranged  | Body 4 · Power 6 · Speed 3                       | Barrage, Longshot, Suppress |
| **Vow**   | Support (Reserve)        | neither | Body 4 · Mind 4 · Speed 3                        | Ward, Mend, Haste           |
| **Hex**   | Controller (Reserve)     | ranged  | Body 4 · Power 2 · Speed 3                       | Confuse, Slow, Dread        |

**Card sketches** (each names the one core rule it bends, §"Cards may supersede the core"):
- **Phalanx** — a stacked lane shares one Focus pool when blocking.
- **Bodyguard** — block a slip in an *adjacent* lane (block beyond your own lane).
- **Taunt** — enemy slips in adjacent lanes must come through Anvil's lane.
- **Blitz** — first slip each round is free (Tempo 0).
- **Shadowstep** — slip a *stacked* lane (ignore one extra blocker).
- **Backstab** — bonus damage vs a Reserve target.
- **Barrage** — one ranged attack hits several front targets.
- **Longshot** — ranged may reach an enemy *Reserve* this round (sanctioned sniper exception).
- **Suppress** — reduce a target's Tempo next round.
- **Ward** — grant an ally a *melee* attack this round (lets a caster self-defend, §4.2).
- **Mend** — restore Body to an ally.
- **Haste** — grant an ally Tempo.
- **Confuse** — strip a target's Focus (it cannot block this round).
- **Slow** — reduce a target's Speed (cheaper to block/engage).
- **Dread** — Fear (§2) forces an enemy Vanguard to refuse its lane.

## Enemies & tutorial dummies

Algorithmic dummies (deterministic scripts that punish the wrong play, reward the right one) for
tutorials; randomized real foes for scenarios. Seeded as content lands.

## Tutorials — base first, Clash module last

Each is one lesson, algorithmic.

1. **The Trade** — same-range melee; both deal base; armor/toughness decides. *Bring effective
   power; armor matters.*
2. **Right Tool, Right Range** — melee/ranged/auto-hit. A ranged foe auto-hits a melee hero who
   can't answer. *Match the range or just eat hits.*
3. **The Triangle** — Vanguard ▸ Skirmisher ▸ Reserve. *Pick the role that counters the threat.*
4. **Lanes & Concentration** — commit a count, assign, **stack** to overwhelm. *Concentrate; an
   even spread loses the stacked lane.*
5. **Slip & Wall** — Tempo to slip vs Focus to block. *Overwhelm a thin wall; hold to deny.*
6. **Guard the Caster** — your fragile Reserve is assassinated if unwalled. *Protect the
   keystone.*
7. **Clash module** (optional track) — the six existing Clash lessons (Interrupt the Wind-Up,
   Read the Lead, Catch the Dodger, Survive the Brawler, The Feint, The Duelist), now taught with
   the Clash module switched on, atop a same-range duel.

## Scenarios

- **Cooperation — teaching combos (2 heroes):** *Ward synergy* (Vow + Sear), *Confuse→slip*
  (Hex + Wisp), *Hold & rain* (Anvil + Sear).
- **Cooperation — capstone (the cascade):** all five (Anvil/Wisp/Sear/Vow/Hex). Coordinated:
  Anvil walls Vow's lane; Hex confuses/slows; Vow wards Sear; Sear+Hex delete the front; Wisp
  decapitates the enemy support. Uncoordinated: enemy Wisp assassinates your Vow → Sear unwarded
  → cascade collapse → slaughter. *Pull one keystone and the deck unravels.*
- **God-tier:** a solo **both**-attack elite vs a swarm — must use range coverage and lane
  blocking to avoid being overwhelmed.
- **Versus:** hotseat lane battles (the §4 commitment system, two human sides).

## Implementation notes (what the engine must do)

- Actor gains an **attack profile** (melee/ranged/both/neither) and a declared **role
  intent**; lanes/phases/roles per §4; resolution = trade / auto-hit (§4.2), with the Clash
  (§1.0) as a switchable same-range resolver.
- Round = §4 declaration cycles (count → assignment → hold/slip → skirmisher targets → reserve
  targets) interleaved with the three resolution phases; order-independent per phase.
- Count-adaptivity (§4.1): present a choice only when it has ≥2 legal options, so 1 v 1 is the
  plain engagement.
- Keep `engine` Bevy-free; the renderer stays generic over `engine::Game`.
