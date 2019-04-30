
# Octothorpe

Turn your APC40 into a step sequencer


### TODO 
Patterns
- [ ] Pattern click activates / deactivates note with current velocity
- [ ] While holding first note down, change note length by clicking a following note in the same key
- [X] row 0x32 -> change pattern length
- [ ] shift + row 0x32 -> change velocity level
- [X] row 0x31 -> change zoom level
- [ ] shift + row 0x31 -> move zoom viewport
- [ ] bank select moves viewport in all directions, also moving zoom indicator
- [ ] scene lauch row selects patterns

Phrases
- [ ] Render phrases

Instruments
- [X] track selection row switches between instuments
- [X] master switches between first & second group of 8 instuments
- [X] row 0x30 (record arm) -> instrument outputs yes/no

Sequences
- [ ] Pan, send A etc. show corresponding sequence
- [ ] Queue sequence by shift clicking sequence button
- [ ] Queued sequence starts after sequence hits a common denominator for all playing phrases

Tempo
- [ ] Fix tap tempo

Effect knobs
- [ ] Send knobs input to output directly for channel of selected instrument
- [ ] Record knobs into playing pattern

Improvements
- [ ] Create one playable trait for pattern / phrase so we can treat them as one and don't have to use match everywhere
- [ ] Don't check every note against the cycle

### Idea / unsure about
Velocity
- Show velocity of notes in grid by color of note?

Effect knobs
- Record effect knobs into patterns
