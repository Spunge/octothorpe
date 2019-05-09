
# Octothorpe

Turn your APC40 into a step sequencer


### TODO 
Patterns
- [X] Pattern click activates / deactivates note with current velocity
- [X] While holding first note down, change note length by clicking a following note in the same key
- [X] row 0x32 -> change pattern length
- [ ] row 0x30 -> change velocity level
- [X] row 0x31 -> change zoom level
- [X] bank select moves viewport in horizontally, also moving zoom indicator
- [X] bank select moves viewport vertically
- [X] Indicator shows in currently playing pattern
- [ ] Use pattern indicator as indicator of base note offset for a second, switching back to selected pattern
- [X] Notes in grid shouldn't be able to overlap, shorten previous note

Phrases
- [X] Render phrases
- [X] Toggle phrases
- [X] Handle changing pattern length in phrases by keeping pattern end around
- [X] Play notes in pattern for PlayedPatterns that are longer as pattern
- [ ] Indicator shows progress in currently playing phrase
- [X] Phrases shouldn't be able to overlap, shorten previous phrase
- [X] Make phrases red instead of green

Instruments
- [X] track selection row switches between instuments
- [X] master switches between first & second group of 8 instuments
- [X] scene lauch row selects patterns in pattern view
- [X] scene lauch row selects phrases in phrase view
- [ ] Copy playables by holding playable key & clicking other playable key
- [X] Shift + playable clears pattern / phrase
- [ ] Keep base note value of grid in instrument, not per pattern

Sequences
- [X] Sequences are played
- [X] Pan, send A etc. show corresponding sequence
- [X] Queue sequence by shift clicking sequence button
- [ ] Show active sequence by blinking sequence light in sequence grid
- [X] Queued sequence starts after sequence hits a common denominator for all playing phrases
- [X] row 0x32 -> instrument outputs yes/no
- [X] Make indicator lights light up for notes played by instrument
- [X] Don't switch sequence on shift click

Tempo
- [ ] Fix tap tempo

Effect knobs
- [ ] Send knobs input to output directly for channel of selected instrument

Improvements
- [X] Create one playable abstraction for pattern / phrase so we dont have to write zoom / length / etc. code twice
- [X] Don't check every note against the cycle
- [X] Don't send same note on message multiple times to controller when grid is zoomed out on large patterns
- [ ] Save state to file
- [ ] Move grid not instantly?

### Idea / unsure about
Patterns / Phrases
- We now have 5 phrases / patterns, 1 under every scene launch button, we could implement shift+up/down for scrolling through more of these
- Fold pattern grid to notes in key / all notes
- shift + row 0x31 -> move zoom viewport

Velocity
- Show velocity of notes in grid by color of note?

Effect knobs
- Record effect knobs into patterns, how do we want to do this, holding rec key?
