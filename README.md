
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
- [ ] bank select moves viewport in horizontally, also moving zoom indicator
- [ ] bank select moves viewport vertically
- [ ] Indicator shows in currently playing pattern

Phrases
- [ ] Render phrases

Instruments
- [X] track selection row switches between instuments
- [X] master switches between first & second group of 8 instuments
- [X] row 0x30 (record arm) -> instrument outputs yes/no
- [ ] scene lauch row selects patterns in pattern view
- [ ] scene lauch row selects phrases in phrase view

Sequences
- [ ] Sequences are played
- [ ] Pan, send A etc. show corresponding sequence
- [ ] Queue sequence by shift clicking sequence button
- [ ] Queued sequence starts after sequence hits a common denominator for all playing phrases

Tempo
- [ ] Fix tap tempo

Effect knobs
- [ ] Send knobs input to output directly for channel of selected instrument

Improvements
- [ ] Create one playable trait for pattern / phrase so we can treat them as one and don't have to use match everywhere
- [ ] Don't check every note against the cycle
- [ ] Don't send same note on message multiple times to controller when grid is zoomed out on large patterns

### Idea / unsure about
Patterns / Phrases
- We now have 5 phrases / patterns, 1 under every scene launch button, we could implement shift+up/down for scrolling through more of these

Velocity
- Show velocity of notes in grid by color of note?

Effect knobs
- Record effect knobs into patterns, how do we want to do this, holding rec key?
