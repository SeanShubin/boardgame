# Log Driven Exploration

Here I am exploring combat mechanics by vetting that the combat logs represent my combat system

## Systems this game is going to have
- All persistent state represented by cards
- Cards in the table can be flipped over
- Cards will be arranged in decks, order matters
- There will be cards that represent attributes
  - The header card will have the attribute name
  - Different attributes may have magnitude, quantity, both, or neither
- During gameplay, generic cards will be laid out in according to quantity for attribute that have state (flip over when state changes)
- A card does not flip over until sufficent magnitude of the same attribute is reached
- The game will be tuned and balanced with no randomness
- There will be mechanics with randomness and hidden infromation, but those mechanics can be toggled, and won't affect game tuning

## Balance
- The game will be balanced according to /docs/game-theory
- Right now I am balancing armor and weapons
- Eventually we are going to have 5 suits of powers (Wall, Infiltrator, Artillery, Controller, Support), as well as passive stat bonuses
- These systems will interlock by being able to use 1 power of each suit + our weapon attack.
- So on a single combat round, we make use of our armor when we get hit, our weapon when we strike, and up to 5 powers
- So I want a robust, balanced, and comprehensible weapon/armor system and go from there
- There are going to be two types of progression.  Vertical progression, where we want as many distinct strategies as possible.  And horizontal progression, where we will be able to switch between suits, armor sets, and weapon sets at will.  There will also be a concept of a god character, that eventually breaks the balance because he has multiclassed everywhere.  This is intentional, the builds need to be distinct precisely to motivate the god to collect them all.

## Combat System
- there are 3 different times spells/abilities/powers can be used (we are talking about the Iron, Silver, Brass, Bone, and Salt Suits)
- each spell will indicate during which of the 3 phases it can be cast
- it is ok for spells to be castible during multiple spell phases, but there is still a limitiation of one spell per suit per combat round
- formation
  - declare intentions/groups
    - intentions
      - vanguard: protects support
      - skirmisher: attacks opposing reserve
      - support: damages opposing vanguard, hinders opposition, helps allies 
    - groups
      - actors on the same side can be grouped together
      - must all target the same thing at any one time
      - vulnerable to area of effect
      - may choose how damage is allocated among them
      - may choose who is the victim of a single enemy effect
  - simultaneous reveal
- declare melee targets, cast spells
- check for skirmishers pushed back into vanguard via interception, they get new targets
- melee combat
- reserve may target vanguard and skirmishers, skirmishers may target vanguard, skirmishers, reserve
- resolve fast range and spells
- resolve melee (skirmishers)
- resolve slow range and spells
