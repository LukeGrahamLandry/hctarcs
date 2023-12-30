
## try running tres (Dec 29)

Cheating and just printing name of costume if it's a char and ignoring pen stamp because that's how they print. 

Now it actually runs and prints: Panic! resolve-path: Could not resolve path `/bin/shell`
(which, for clarity, is a fake os panic not a rust panic).
Problem is `Str::from("/") == path.get_index(self.local_id_649_i)` is never true because I derived eq on my fancy new string type.

Fixed that and now real panic (but later): `Tried to send invalid computed message: Owned("_switch_heap_deallocate_Vec")`
Which I guess is just that scratch silently ignores invalid computed event names and not every type registers a destructor listener. 

Ok, now fake panic `Panic! interp-step: ip 13 is out of bounds`. 
Fair enough, on real scratch it seems to only get to 7 without input. 
Seems its never actually dispatching a switch-interp event. 
Again, I think the problem is `self.local_id_454_curr_token.clone().as_num() == (0.0f64 + self.local_id_454_curr_token.clone().as_num())`
Which is supposed to be checking if it's a number or string but perhaps confused my type inference.
If I manually replace that condition with `matches!(self.local_id_454_curr_token, NumOrStr::Num(_))`
the interp works correctly on `"hi" println` but on `1 sin println` it says: Panic! interp-step: unknown instruction `1` 
Manually replacing the condition with 
```
{match &self.local_id_454_curr_token {
    NumOrStr::Num(_) => true,
    NumOrStr::Str(s) => s.as_ref().parse::<f64>().is_ok(),
    NumOrStr::Bool(_) | NumOrStr::Empty => false,
}}
```
runs the sin example but always returns 0 which makes sense if it's treating my string as 0.

I'm very excited to port my postscript mandelbrot to this and see if mine is faster than turbowarp. 
Tho seems there's no `roll` so maybe can use a vec instead of the stack, so you get less painful random access. 
Learning loops: `0 1 2 3 4 5 6 :loop dup println 0 > ":loop" jump-if` prints those numbers. 
But that doesn't work on the version on scratch website? confusing. 

Pretty sure I decided all strings coerce to zero because `"1"` did but that's probably actually the string with the quotes. 
I can have optimisation that recognises the idiom of `(x == (0 + x)) === x.is_num()` and just call a runtime method. 
It's cool that I can emit code that shows intent more clearly (as long as there's no possible other interpretation for that expression). 
For now, it's a hack because I only recognise that specific shape of tree and miss-compile if I don't notice. 
TODO: test flag that enables and disables opts like that, so I can have little sanity checks that they behave the same. 

TODO: test flag that reverses iteration order. 
TODO: little scratch programs that assert sanity checks. 

Mistakes I made:
- missing else branch on if
- unit statement
- expression statement

## argument type inference (Dec 29)

So I need a way to link the expressions used as arguments when calling a procedure to the parameter variables used in the body.  
Currently I'm just emitting procedures in sequence all at once, but I need to do a pass first that just notices which exist, 
so I know there's a VarId already declared for the parameters when I try to call one.

That works and I can infer types, but now I have to allow arguments to be polymorphic as well because
vec_push(value) can be a num or a string. So I need that as more of a first class idea where if an expect_type fails,
it can change to a polymorphic one. But I feel this will get scary and order dependent because what if someone else 
inferred their type based on your incorrect guess. 

TODO: have a borrowed from of to_str for checking equality from a list, etc.

what happens when you put a bool in a list? does it just become the string? Yep. 
Ok so bool is a valid polymorph type I guess. 
cause then I can notice that if you're checking equal to the string true and do it faster. 

On an unrelated note, i figured out the problem where message names had duplicates.
problem is that multiple names can match same safe_str after remove special characters.
easy fix by adding a VarId in the mix and make sure not to use `safe_str` in the generated match branches of `msg_of(String)`.

Fixed a few more inference places, and now I'm down to one last problematic expression: `self.local_id_454_curr_token.clone() == (0.0f64 + self.local_id_454_curr_token.clone())`.
What does adding a number to a string do? I think even number strings don't coerce so that's checking if it's a number. 
`"1" -> false, 1 -> true, "aaa" -> false`  
which is a great candidate for optimising because I'm already tracking type info, but anyway, for now I think just any string is zero.
But it thinks it's a string because you only use it as that so my inference assumes it's safe to coerce to string from the heap read
but actually, it is polymorphic and that check is done at runtime.

Alas, now I've fixed enough errors that the borrow checker actually runs, and it hates when you use length of list on both sides of an assignment. 
Lucky that's a statement, so I can put locals. 

Passes cargo check!

TODO: test that makes sure any changes never make the ray tracer use polymorphic types. (I had to rescue bools once already)
Currently my type inference is non-deterministic, so I think whether it compiles the ray tracer 
correctly depends on the (random) iteration order of a hashmap.  
So the common problem it seems is locals that are only ever assigned from arguments 
and passed to functions without being directly used in an expression so those statements 
could be processed before it knows types for the function parameters, and then it guesses 
they must be polymorphic. Instead of guessing, I should walk the tree and see if there's any usages that have type checked by now.
Did that, feels like a lot of cloning that would be slow, but also it makes no difference cause profiler says 75% of the time is serde parsing the json anyway (or 86% in debug mode).

Can also assert cargo check passes as part of the tests. 
And if I can think of any dumb syntactic things to check for like `NumOrStr::from(NumOrStr::from(...expr))`.
TODO: equality check without cloning the string 

Feel the need to make my own string type, so it doesn't have to allocate when getting a char, just store that inline. 
I'm sure someone has a better version of small string optimisation I could depend on. 
But have to make sure it also doesn't allocate for constant strings.

String get letter is zero indexed! So are lists! Same with for-each-loops!

## async ideas (Dec 28)

Do my own async, so I don't have to deal with functions closing over their mutable sprite/global arguments? 
I'm not sure how you'd express the idea of "yes this future needs unique access to something,
but it releases it when it yields and the caller will return it when it resolves" in rust's async model.
I think I want my runtime thing to be in charge so when you await WaitSeconds or whatever, it jumps back out into my code. 
The more interesting case being BroadcastAndWait where I need to go back into the World, and it needs access to all the sprites. 


## lists & type inference (Dec 28) 

tres actually uses different types, so I cant avoid anymore. 
As a starting point, what if we assume thing are never actually polymorphic. 
So when you see an opcode using a variable, infer a type for it. 

Set variable: infer value type and expect var type to be that. 
Increment variable: expect value and var to be number. 
I'm treating the strings true and false as boolean literals which is wrong and should be fixed I guess. Have an opt pass that notices that idiom. 

Alas, tres heap list is polymorphic. 

The index field can be "last" or "random" which is weird. 
- https://en.scratch-wiki.info/wiki/Item_()_of_()_(block)

TODO: im being inconsistent. get/set list takes scope as an arg but get/set other var has two ast nodes.  
      lists aren't expressions (which maybe is right since not first class in scratch).

LetterOf seems like poor naming, its "take a subslice of one index of the string". 
But sounds to me like its asking a contains question. lucky me, they have a different shape for bools. 

Damn where on earth are they getting a control_for_each. Its fully not in the menu as an option but it clearly works. 
Oh, okay there are just some secret magic blocks I guess. That's crazy man. 
Also, strange naming its `for a in range(b): c` not `for a in b: c`
- https://en.scratch-wiki.info/wiki/For_Each_()_in_()_(block)
- https://en.scratch-wiki.info/wiki/Hidden_Blocks

Seeing the comment `; Iterate in reverse so processes can be removed without interfering with the next iteration` 
does not bode well for my borrow checker ;-;

Sadly for my enum plants broadcast values can be a computed string. 
Tho the handlers have to be a constant from the drop-down. 
So I guess if you compute it, I just switch over it and crash if you commit a crime. 

Since the program I care about rn only has one sprite, I can cheat at broadcast_and_wait

## contributing to the lisp one (Dec 28)

Implicit locals: 
Procedure.variables has them and the cranelift backend defines them as real locals.   
serialize_proc puts them in SerCtx.local_vars
serialize_sprite (where instance vars are handled) doesn't put locals on the sprite or stage.
Is there anything that checks that you always assign to a local before you read it? Otherwise, they're actually statics which is scary once sprites can clone. 

Damn git submodules suck ass, apparently. I just wanna test my thing! 
- https://stackoverflow.com/questions/20929336/git-submodule-add-a-git-directory-is-found-locally-issue

## Ideas (Dec 27)

Next test I want is https://scratch.mit.edu/projects/647528063/editor/
Needs broadcasts, lists, and costumes stamp.
Other complicated projects to try are the featured ones from https://turbowarp.org/

TODO: could have easy way to fetch project by id. 
- https://api.scratch.mit.edu/projects/ID has a `project_token` field
- https://projects.scratch.mit.edu/ID?token=TOKEN has the project.json file
Idk why they have that split, both seem to be fine being fetched by curl. 

TODO: implement some turbowarp extensions
- function return values! https://docs.turbowarp.org/return

Supporting wasm would be funny.
Turbowarp's compiler/runtime is actually pretty big, so I might be able to make a lighter embed. 
Canvas render backend is probably easier than a native one. 

Pleasing that RustRover's profiler works because it's just a normal rust project in the end. 
TODO: Generating random numbers is like ~2/3 of the Sprite::receive
      And you can't put the rng anywhere cause of the mutability thing. Aaaaaarhg!

Web UI where you can upload a .sb3 file, and it will spit out the rust code. 
You'd still need the native rust compiler, but it would be a cool demo. 

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

Start with softbuffer as the renderer. Can cheat because this is (i assume) using lines to draw individual pixels. 
wgpu changed a bunch of random stuff since last I used it. 

Transform to right coordinate system
Very close to right, some stuff is kinda upside down. Wrong trig units? 
Not the man problem but kinda helped take a line off the sphere, but now it's slow!
I was converting the output of `a<trig>` to degrees as well instead of the input 
and fixing that didnt fix the upside down but did fix the slow so now its fast again.
Which is pretty cool I think. llvm must know identities for the trig intrinsics?
I feel the problem must be somewhere in dielectrics refraction. 
- https://raytracing.github.io/books/RayTracingInOneWeekend.html#dielectrics/refraction

## Variables & Emit Rust (Dec 27)

- argument values are immutable in the body, no local variables
- variables are either global or instance fields on a sprite 

Does `stop this script` mean 
- A) break from current scope 
- B) return from the current custom block 
- C) stop this thread (all the way up the stack to the initial trigger)

TODO: the lisp compiler doesn't declare local variables anywhere. Importing to scratch and exporting puts them on the stage as globals tho.  
      
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
