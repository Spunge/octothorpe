
# Octothorpe

Turn your APC40 into a step sequencer


### TODO 
Patterns
- [ ] Pattern click activates / deactivates note with current velocity
- [ ] While holding first note down, change note length by clicking a following note in the same key
- [X] row 0x32 -> change pattern length
- [ ] row 0x30 -> change velocity level
- [X] row 0x31 -> change zoom level
- [X] bank select moves viewport in horizontally, also moving zoom indicator
- [ ] bank select moves viewport vertically
- [ ] Indicator shows in currently playing pattern

Phrases
- [X] Render phrases
- [ ] Handle changing pattern length in phrases by keeping pattern end around
- [ ] Indicator shows progress in currently playing phrase

Instruments
- [X] track selection row switches between instuments
- [X] master switches between first & second group of 8 instuments
- [X] scene lauch row selects patterns in pattern view
- [X] scene lauch row selects phrases in phrase view
- [ ] Copy playables by holding playable key & clicking other playable key

Sequences
- [X] Sequences are played
- [X] Pan, send A etc. show corresponding sequence
- [ ] Queue sequence by shift clicking sequence button
- [ ] Show queued sequence by blinking sequence light in sequence grid
- [X] Queued sequence starts after sequence hits a common denominator for all playing phrases
- [X] row 0x32 -> instrument outputs yes/no
- [ ] Make indicator lights light up for notes played by instrument

Tempo
- [ ] Fix tap tempo

Effect knobs
- [ ] Send knobs input to output directly for channel of selected instrument

Improvements
- [X] Create one playable abstraction for pattern / phrase so we dont have to write zoom / length / etc. code twice
- [X] Don't check every note against the cycle
- [ ] Don't send same note on message multiple times to controller when grid is zoomed out on large patterns
- [ ] Clear pattern / phrase button


### Idea / unsure about
Patterns / Phrases
- We now have 5 phrases / patterns, 1 under every scene launch button, we could implement shift+up/down for scrolling through more of these
- Fold pattern grid to notes in key / all notes
- shift + row 0x31 -> move zoom viewport
- Make it possible to offset pattern start in phrase
- Use pattern indicator as indicator of base note offset?

Velocity
- Show velocity of notes in grid by color of note?

Effect knobs
- Record effect knobs into patterns, how do we want to do this, holding rec key?
