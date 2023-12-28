
## Rendering Pen (Dec 27)

To start with, lets just collect all the lines it draws. 

Walk through the list of triggers and create an enum. 
Then generate a handler function that matches a message and runs the code. 

It runs but infinite loops. 
The raytracer starts by GoTo(-Infinity, Infinity) and the calculates screen size based on its coordinates,
so I guess scratch clamps you? Yep, that worked. TODO: try to optimise that out sometimes? Or just hope llvm does it I guess. 
- https://en.scratch-wiki.info/wiki/Coordinate_System

Logging the count, that drew 173280 lines compared to the (expected? optimal?) 480x360=172800 = one per pixels. 
Well, that's an extra 480 lines, so it's just counting the reset every row. 

Once this works, next test is https://scratch.mit.edu/projects/647528063/editor/
Needs broadcasts, lists, and costumes stamp. 

## Variables & Emit Rust (Dec 27)

- argument values are immutable in the body, no local variables
- variables are either global or instance fields on a sprite 

Does `stop this script` mean 
- A) break from current scope 
- B) return from the current custom block 
- C) stop this thread (all the way up the stack to the initial trigger)

TODO: the lisp compiler doesn't put global variables on the stage. Importing to scratch and exporting puts them tho. 
TODO: default values
TODO: check for variable name conflicts 

Maybe it was a dumb idea to try to parse out every opcode into an ast node because most are just going to be calling 
a function in the runtime. So there would be a lot of redundant dispatching logic. Instead, I could just define 
the prototype each expects, parse the argument types in one place, and define functions with the right names in the runtime. 
I think I still want control, variable, and expression blocks to have their own nodes, so I can try to emit sane looking code. 
But things like pen, looks, sounds, motion are probably best expressed as a function call anyway. 

TODO: 
  - How to represent which builtins are await points?
  - Is everything a method on Sprite? 
  - How to represent inheriting the base sprite behaviour? 

Very nice to just emit unknown stuff as a `todo!(<src>)` so I can see progress while working on it. 

I'm making custom blocks be a method on a struct that takes the sprite data and the globals as well as whatever arguments. 
That's a bit clunky. 

Got rid of unop ast nodes for math functions, now just have a string for suffix method call. 

Annoying that argument_reporter_string_number only has readable name and procedures_call only has id. 
Actually it doesn't matter because they're ordered, nvm.

TODO: type inference for string/num/bool variables. no bool variables but could recognise patterns like a string that is always true/false or number 1/0 always compared. 
      doing hacky things rn and just treating "true" as 1.0 and "false" as 0.0 so everything can stay floats. 
      need to fix this! 

It compiles! 
TODO: parse colour arg, render pen, main entry point. 

## Parsing Ast (Dec 26)

So it has map of blocks with next pointers. Find ones with entry point opcodes and then follow the linked list to build up the stack.
`inputs` is a map of named arguments. Math has `NUM1` and `NUM2`, logical has `OPERAND1` and `OPERAND2`, etc.
It would be very nice if I could express this to serde instead of manually writing a bunch of code that pulls it out of the map. 
- https://serde.rs/enum-representations.html#untagged

Then for custom blocks:
- procedures_definition: 
  - entry point for a stack.
  - `inputs[custom_block]` is a procedures_prototype.
  - `next` is the first block in the body.
  - When the body wants to reference an argument it uses another `argument_reporter_string_number` block?
- procedures_prototype: 
  - `inputs` keys are local arg names as used in inputs of procedures_call .
  - `inputs` values are argument_reporter_string_number.
  - no next.
  - `mutation` field with extra info
    - `argumentids` match input keys
    - `argumentnames` match `fields[0]` of corresponding argument_reporter_string_number
    - `proccode` gives a string for the function name which I think is how procedures_call references it instead of by block id 
      - Arguments have types, `%b` for bool and `%s` for number/string (so some inference needed)
- procedures_call
  - another `mutation` field 
- argument_reporter_string_number: 
  - `fields[VALUE]` is name of the argument

Rust macros can't expand to one branch of a match??? And the error message for that is 
`macro expansion ignores token `=>` and any following. Macro <NAME> is likely invalid in pattern context`,
which is true I guess but did not make it obvious to me that what I wanted was impossible rather than just me making a syntax mistake. 
I guess a macro doesn't expand to a stream of tokens, it has to expand to whole ast nodes? 
But like ehhhh why, sad day. 

## Indirect Lisp Idea (Dec 26)

- https://github.com/Johan-Mi/scratch-compiler
- https://github.com/Johan-Mi/sb3-builder
- https://github.com/Johan-Mi/unsb3
- https://github.com/Johan-Mi/linrays

Found something fucking amazing! Compile lisp to a scratch project. 
Can use this for testing my thing without needing to write everything in the gui. 

This leads to the idea of the most indirect lisp compiler: take your lisp and compile it to a scratch project 
with their thing and then compile that scratch project to rust and then compile that. 

They also have a native backend using cranelift (but no frontend accepting sb3 that I can see).
It looks like it doesn't have graphics or concurrent sprites yet tho.
Should see if I can hook that in with my runtime thing if I ever get that far. 

## Ast Chore (Dec 25)

I'm not awake enough to do any real work, so I figure I'll do some drudgery of
transcribing nice types for all the different blocks.

There's pretty much the normal programming language structure.
- Statements that perform some action to the simulation state. 
- Control structures that operate on blocks of statements.
- Different expression operations that return values. 

There are some kinda interesting representation choices. 
- Should custom blocks be inlined? I guess not, it would be very cool if the generated code was readable. 
  Can you do recursion? 
- Distinguish between time builtins, pos/dir, local properties, globals?
- Are math functions unary operators? Why is `not` more special than `sin`?
- Should the AST match the blocks or should await points be represented in some special way? 
  (places where its split across frames, like glide animation)
  Maybe this introduces another form of IR.   

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
