
## Ast Chore (Dec 25)

I'm not awake enough to do any real work, so I figure I'll do some drudgery of
transcribing nice types for all the different blocks.

There's pretty much the normal programming language structure.
- Statements that perform some action to the simulation state. 
- Control structures that operate on blocks of statements.
- Different expression operations that return values. 

There are some kinda interesting representation choices. 
- Should custom blocks be inlined? I guess not, it would be very cool if the generated code was readable. 
- Distinguish between time builtins, pos/dir, local properties, globals?
- Are math functions unary operators? Why is `not` more special than `sin`?

## New Beginnings (Dec 25)

- https://scratch-tutorial.readthedocs.io/fr/latest/1_intro/intro.html

Goal: compile a scratch project to a native executable. 
I briefly started this a while ago but want to consider coming back to it. 

How to get the project file:
- Open in the scratch editor: `https://scratch.mit.edu/projects/ID_NUMBER_HERE/editor/`
- Menu bar at the top: File > Save to your computer
- Rename the downloaded *.sb3 file to a *.zip and unzip it
- It should contain *.svg (textures), *.wav (sounds), project.json (code)

My idea for first step is parse a project file into a meaningful AST and dump that to an s-expression 
format that has less redundant info (unique ids, positions, etc.) so its less awkward to commit for tests. 

Seems like actually compiling it should be cool and different from other languages 
because they've got a very event driven / message passing thing going on. 
