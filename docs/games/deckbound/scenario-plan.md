# Deckbound — Scenario & Tutorial Plan (build blueprint)

> **STALE — read for content *intent* only (re-sync banner 2026-06-26).** This blueprint predates the
> current model on several axes; **canon (`canon/2-spec` §4 / §4.6) + `booklet.ron` are authoritative**:
> - **Stats:** the five are **Might · Vitality · Toughness · Cadence · Finesse** (not Speed / Drive /
>   Daring / Power / Mind); one damage channel (untyped Might → health); Fear / Armor gone; ranged
>   attacks **evadable** (§2 / §3).
> - **Combat:** the §4.6 **six-phase** model (Standoff → Fray → Volley → Breach → Reckoning → Lull) —
>   **lanes / stacking / the Outrider rank / the gauntlet / the crossing contest are retired**; the back
>   is reached by a **per-unit-lock breach** (a free Vanguard charges in the Volley), not a lane crossing.
> - **Tempo:** one pool, **Cadence × Finesse**, shared across the round's phases (Focus / Mind merged out).
> - **Role cards (in code):** no per-suit/per-side cap — casting is **tempo-gated**; offensive (foe-
>   targeting) cards are positioned by reach, support is rank-free (§4.4). The "seven crossing powers"
>   below (Phalanx / Taunt / Blitz / Shadowstep / Backstab / Longshot / Bodyguard) are **retired flavor**
>   pending the §4 role-power **open dial**.
> The tutorial / scenario *intent* below (what each lesson teaches) still guides authoring; the
> mechanics vocabulary does not.

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

| Name      | Role                   | Attack  | Profile (seed)                                   | Signature cards             |
| --------- | ---------------------- | ------- | ------------------------------------------------ | --------------------------- |
| **Anvil** | Wall (Vanguard)        | melee   | Body 10 · armor heavy · Mind 5 (Focus) · Speed 2 | Phalanx, Bodyguard, Taunt   |
| **Wisp**  | Infiltrator (Outrider) | melee   | Body 4 · Speed 7 · Power 4                       | Blitz, Shadowstep, Backstab |
| **Sear**  | Artillery (Rearguard)  | ranged  | Body 4 · Power 6 · Speed 3                       | Barrage, Longshot, Suppress |
| **Vow**   | Support (Rearguard)    | neither | Body 4 · Mind 4 · Speed 3                        | Ward, Mend, Haste           |
| **Hex**   | Controller (Rearguard) | ranged  | Body 4 · Power 2 · Speed 3                       | Confuse, Slow, Dread        |

**Card sketches** (each names the one core rule it bends, §"Cards may supersede the core"):
- **Phalanx** — a stacked lane shares one Focus pool when blocking.
- **Bodyguard** — block a slip in an *adjacent* lane (block beyond your own lane).
- **Taunt** — enemy slips in adjacent lanes must come through Anvil's lane.
- **Blitz** — first slip each round is free (Tempo 0).
- **Shadowstep** — slip a *stacked* lane (ignore one extra blocker).
- **Backstab** — bonus damage vs a Rearguard target.
- **Barrage** — one ranged attack hits several front targets.
- **Longshot** — ranged may reach an enemy *Rearguard* this round (sanctioned sniper exception).
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
3. **The Triangle** — Vanguard ▸ Outrider ▸ Rearguard. *Pick the role that counters the threat.*
4. **Lanes & Concentration** — commit a count, assign, **stack** to overwhelm. *Concentrate; an
   even spread loses the stacked lane.*
5. **Slip & Wall** — Tempo to slip vs Focus to block. *Overwhelm a thin wall; hold to deny.*
6. **Guard the Caster** — your fragile Rearguard is assassinated if unwalled. *Protect the
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

## Implementation status (this build)

- ✅ Engine on the §4 lane system (Assemble → **Assign** → Slip → Vanguard → Outrider → Rearguard),
  same-range trade + range auto-hit (§4.2), optional 1v1 Clash module, count-adaptive.
- ✅ **Manual lane assignment** (stacking) — offered when ≥2 lanes and ≥2 Vanguard.
- ✅ **All seven powers wired as passive abilities** (detected by card name): Phalanx (combined
  block), Bodyguard/Taunt (guardian lends Focus to other lanes), Blitz (free slip), Shadowstep
  (ignore one blocker), Backstab (bonus vs a foe Rearguard), Longshot (Rearguard may target foe
  Rearguards). Plus the played effects: Ward/Mend/Haste/Rally/Steel/Confuse/Slow/Suppress/
  Barrage/Dread/Cleave/Sunder/Bank.
- ✅ Foe-side **rearguard targeting matrix** enforced (heroes' Rearguard fire hits the foe front;
  Longshot or an empty front reaches foe Rearguards).
- ✅ **Hotseat PvP lane driver** — both sides human, pass-and-play, hidden commit per phase
  (Assemble → **Assign** → Slip → Outrider → Rearguard, committing side alternates). Versus holds
  3v3/2v2 PvP lane battles plus a 1v1 Clash duel.
- ✅ **PvP manual lane stacking** — with ≥2 lanes and ≥2 Vanguard, *each* side now places its own
  lanes by hand (the device passes A → B for the Assign phase) and may stack a lane, the same
  count-adaptive choice PvE has. (Was the remaining first-pass refinement.)

## Implementation notes (what the engine must do)

- Actor gains an **attack profile** (melee/ranged/both/neither) and a declared **role
  intent**; lanes/phases/roles per §4; resolution = trade / auto-hit (§4.2), with the Clash
  (§1.0) as a switchable same-range resolver.
- Round = §4 declaration cycles (count → assignment → hold/slip → outrider targets → rearguard
  targets) interleaved with the three resolution phases; order-independent per phase.
- Count-adaptivity (§4.1): present a choice only when it has ≥2 legal options, so 1 v 1 is the
  plain engagement.
- Keep `engine` Bevy-free; the renderer stays generic over `engine::Game`.
