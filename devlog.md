
TODO: impl Try for IoAction for functions that only optionally return an io action. 

## TODO: porting tests

It's such a pain to use the gui so need to add everything im using to scratch-compiler. 
Also integration for calling it. 
Also build script for website demos. 

TODO: why are my egui events super sluggish in wasm build? must be something wrong with egui-macroquad cause the egui demo is fine. 

- wait
- days since 2000
- clone, delete this clone
- broadcast (no wait)

- key input
- timer, reset timer
- when sprite clicked 
- mouse down, mouse x, mouse y, key down 
- touching

## Attempt Try Trait (Jan 17)



## Fixing Quicksort (Jan 11)

Fixed missing default value when its a string (list_length was "200000").
TODO: Thier validate function reads off the beginning of the list and relies on empty comparing less than everything, 
including negatives. So empty as a number is negative infinity? But empty minus a number its treated as 0. 
But its a special zero that's lower than anything else. 
Temp hack around that by just starting the loop at 2 and stopping 1 early. 

Average: (mine_release_nolto=0.062s), (mine_release_lto=0.056s), (turbowarp=0.050s), (scratch=5.745s). 
So I'm ~11% slower than turbowarp and ~102x as fast as scratch. 

The sort function isn't even async so improving my compilation there won't help. 
I really hope if I can improve type inference enough to make that list not polymorphic it will get better. 
For this kind of number crunching with no allocation it would make sense to me if v8's jit and llvm were the same. 

To validate that theory, manually replace all the as_num in fast() with trusted_as_num which used 
unsafe unreachable_unchecked if the poly is not the Num varient. Now mine_release_lto=0.051s. 
So that's a worthy quest. Just to make sure, I also tried inline(always) on as_num which was slightly slower than original. Compiler wasn't able to prove the branch was never taken. 
And that's still with each list element being 5 (?) words instead of 1 so 
if I can actually make it a List(f64) there should be some cache locality benifits too. 

```rust
pub fn trusted_as_num(&self) -> f64 {
    match self {
        Poly::Num(n) => *n,
        _ => unsafe { unreachable_unchecked() },
    }
}
```

## Quicksort (Jan 9)

Added tracking of which closures are zero sized. 
Doesn't actually change anything, I just want to see because 
im trying to decide if they should capture state instead of runtime passing it in. 
TODO: maybe do that only for the ones that have to allocate anyway. 

The miscompiled linrays where it thinks fn return is async: 
- ~1.2s (before fut machine, commit: fiddling with ui)
- ~0.5s (with fut machine, commit: track alloc)
(both edited to add `Stmt::StopScript => self.mark_async()`, without that its like 220ms)

And it's still dramatically inefficient. 


I want to get turbowarp's quick sort benchmark working. 
Only change to it, I made was converting the sprites to bitmap because i don't want to deal with svgs yet. 

Found a bunch of bugs just by compiling a new project. 
- found that I allow empty else branch but not empty if, fixed in parser. TODO: more consistent handling in codegen (just treat if as empty else)
- wasnt properly setting scope when accessing globals
- `?` in safe_str
- throught im not inferring for list types but am in parse. not infering for globals in general? 
i was emiting var ids for globals seperately but then parsing it as a normal sprite and re-emitting new ids and some thing were finding the old ones and some the new so the inference didnt work
- repeat was using opt_var instead of opt_block

Added Features
- need to parse default list values.
- sensing_dayssince2000. TODO: use right epoch instead of unix one
- temp ui button for sending click event to sprite. TODO: real bounding box
- emit defaults. rustc really can't deal with my 200k vec literal of floats. had to put it in a string and parse at runtime which feels a bit silly. TODO: use include_bytes instead of parsing a string. 

TODO: seperate file for data (defaults + var names + var getters/setters)
TODO: default costume size for tres 

## Collapsing Futures (Jan 9)

Before i start messing with things clarify the problem statement. 
The way I got it working involved insane double wrapping of everything to be able to pass it to the event loop without running side effects too early. 
Importantly, the observation that while expressions can't create side effects, they can observe them. 
Unfortunatly this sucks. 
- The generated code is completely unintelligible so its really hard to debug. 
- Every block is a new boxed closure. I thought that wasn't too bad since you only allocate if you capture something but since I chain them, if someone at the bottom needs to use a function argument, everyone above needs to allocate.  

The new way to think about it is a user function call returns an iterator that yields io actions. 
But like a rust iterator where it doesnt need to build up the full list first. 
My previous attempt returning a vec of io actions means each sync part needed its own boxed closure to return the code without running yet. 
But now the runtime just calls back into it repeatedly. 
Then need to decide if you make every block its own one of those so like body of loop needs an allocation. 
OR you could have it return the next state to call it with instead of having the runtime implicitly increment the number 
and inline all blocks into the function body switch statement. 
The latter is fewer closures, just reuse the one in the function. I'm already doing most of the work returning break or continue.  
In fact, its extra the same because current loop break returns the whole action after the loop because of how I need to move arguments to inner closures. 
So instead of the current insane nesting, it would be more like a traditional compilation target where everything is a flat chunk of code split into basic 
blocks that jump to each other. And since you'd never do closures, dont have to deal with cloning the captured args out of the fnmut. 
If I do that, it means one allocation per function call instead of one per block. 

Interesting that im slowly approaching how (i think) rust actually implements async functions. 

I think the only thing you lose in this whole adventure is 
slightly less of the upcoming futures are visible on the debugger stack, 
but really I had to do enough nesting to fix data dependencies 
that you didn't really have it before either. 

NOTE: I could try to use rust generators but I think i'll have the same problem as with async where you can't have it relinquish references on yield. 

it gets worse before it gets better. 
separate loop var declares from init. you dont need the =0 now cause it sees that never read before write but that wont be true once its all in a switch stmt. 
TODO: its sad that this makes sync code look dumber too. should fix that. 

holy shit it looks so much nicer and the compiler is so much simpler. 
doesnt work tho lol
- missing a plus one for basic stmt jump so they loop on themselves 
- forgot to return the span for loop so was just counting first stmt as body
ayy fixed. 

so now user functions use state machine. 
TODO: do it for recieve as well
TODO: the state machines are still really inefficent and return IoAction_None a lot just to use the runtime to cycle the switch statement. 

## fixing tres (Jan 7)

> All my baubles for a pure function 

Noticed making it think stop this script is async makes linrays really slow and added a graph of how many futures handled each frame. 
Turns out its 700k... oops. So yeah need to work on that. 
TODO: Should allow in between functions that don't need to do any io actions but call a function that does 
at the end to not be themselves wrapped in a future. 

progress on tres debugging: 
now think problem is not my loops.
find_name_in_directory is putting -1 on the stack instead of 11. so the actual miscompile is in there?
(if all inputs are same)
same input stack. both have "bin" as name args. 
but wait no, thats a sync function, the code is the same. 
so wrong thing in the heap somewhere? 
yeah async has end=0, sync has end=1
sync has `heap[8]=11`, async has  `heap[8]=Empty`
sync is coming in to resolve_path with Vec, 1, 11 on the heap but async has Vec, 0, Empty.
So async didnt read the path?
vec_with_capacity is same with both.
create_fs is where it puts things on the heap.
create_file is the difference? 
async doesnt call push_vec before resolve_path
when async gets to add_ptr_to_directory, dir is always 0.
AYY found problem.
was really obvious idk why im dumb.
a call to a async user function is not closed because the arg values might be expression that change before you reach that point
sadly for my sanity i left a `// TODO: untested` comment right where the problem was. 
so now i really have to sort out the capturing params and passing down chain since every call is wrapped. 

ok ive made the most insane looking code you ever did see 
and it still doesnt work but different problem. now says "system deadlocked". 
also linrays is all black if in async mode so fucked up. 
lol i was still reversing actions list. 
so im back to the same fucking problem now where they dont chain right

TODO: my great nosuspend!() for extra check breaks the formatter. 

TODO: now AskAndWait only works in inspect mode because it uses egui 

## async tres (Jan 6)

Removed the sync version of receive since I don't bother using it ever for fully sync projects. You just wrap the message handler in a single future. (That's not even an allocation since it wont capture anything, its one fn ptr indirection).

Tres is big enough to run into a problem with loops being FnMut so if they capture the function arguments, you can't move out of those for await points in the body or at the end. Noticing this I made everything else FnOnce cause that's easier but doesn't help loops. I'll just clone fn args in loop preamble. Might be able to do better but its only sad for computed strings anyway (lists can't be function args). 

TODO: since I never mutate strings anyway, Str::Owned should be an `Rc<str>` instead of a String. 
Alternativly, I could optimise concat to reuse the buffer but I clone for every call and loop so thats more important. 

Now it builds but just "system has been shut down"
Was problem with my weird closure action chaining in collapse_stmts.
now panic could not resolve path `/bin/shell`

## trying to get browser working again (Jan 6)

It broke the wasm version kinda tho.

```js

// Setup quad url which is needed by macroquad_egui.
// I hoped this call fixes console errors when interacting with widgets. I guess they're expecting different version of api or I just included wrong somehow?
// (wasm_exports.focus is not a function)
miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    version: "0.0.1",
    name: "quadurlidk",
});
```
But no that changed nothing and even without it I get `Plugin quad_url version mismatchjs version: 0.1.0, crate version: 0.1.1`
which isnt what i named my fake plugin so the thing must be registering iteslf properly.

Its blindly setting params_register_js_plugin and params_set_mem without declaring, expecting to already exist?
maybe they're just making sure not to conflict with other scripts that want to use
those names? but why not just put everything in a scope instead of global.

```js
// TODO: i have wrong version on rust side i guess cause this doesnt exist and console complains
if (wasm_exports.focus === undefined) {
    wasm_exports.focus = () => {{}};
}
```
but you cant do that, `TypeError: Cannot add property focus, object is not extensible`

so thats not great sucess but on the other hand it does seem to work and let me interact with the gui.
its just only showing the first frame of the animation. 
so the actual problem is probably just that timers dont work. 


## inspect ui (Jan 5)

want to be able to clear the pen. 
was thinking just pass a Handle in, but then macroquad needs to be set to the pen texture but that's not where iwant to draw the ui to. 
current solution is new concept of events which are like triggers but handled by the world 
instead of sent to all sprites and then just process those every frame when you're in the right context. 
maybe should use those for start/stop/reset too and pass them out instead of getting a mutable world?
but then im just adding a bunch of indirection. somehow ive accidentally made the most hyper object-oriented program. 
like this is DIY dynamic dispatch over the render backend because i just need to adjust timing. 
the ui could return a closure for the action it wants to do next frame i guess.

## turning on the async (Jan 4)

so really it should just be a little tweak in the compiler to emit async entry points and then my basic program should work. 

confusing thing with my wait. 
problem was that my sleep(seconds) function was sync.
but actually it was from_seconds(secs as u64) rounding down 0.5 to 0. 

next: loops dont terminate because im creating the counter in the same closure as the body. 
then still broken, fixed by moving increment into same real rust block as the if of the loop. 
i think actual problem was that close_stmts had a .rev() from when i was using .then i guess 
so it was doing things in the wrong order maybe? but less indirection better, means fewer things need to close over the loop counter. 
tho actually now its resetting my pos to 0,0 at the end which should be the first thing so clearly 
my ioaction chain construction in the compiler isn't working. 
just gonna use seq(vec!) for close_stmts for now. TODO: figure it out and profile to see if it's faster the other way. 

now why am i not rendering anything. aa! default size_frac was 0. 

TODO: implement wait in scratch-compiler so i can try it 
TODO: better debug mode with toggles
TODO: non-turbo mode cause it turns out you want that for debugging 
TODO: really what i want is break points. fuck. this is gonna be hard. 

## templates

The compiler needs several string templates for special files and its getting pretty ugly to have them as constants in the src. 
And when they need vars inserted its extra annoying because format macro needs to take a string literal 
not a string constant, so I've been making a function for each that just takes args and calls format.
Plus, I want to support overriding any file from the cli. 

Maybe better is a folder of files that contain a string literal like a format string that you include and  
first check a hashmap of overrides to maybe use a different filename. 
Was thinking that would suck cause override would need to fmt escape curly braces but no cause dynamic file wouldn't be macro parsed. 
So then default case can still use fast fmt to insert args and custom needs to find+replace. 
Always use named args, and then the compiler checks that my args match the expected ones. 
That's pretty pleasing. 

Unrelated: my great idea of committing the generated frames png, so you can see diffs doesn't work because 
it seems there's random noise in generating the image, so you get changes even when they look the same. 
Too bad cause the github desktop img diff display is very nice. 

## cli direct wasm build (Jan 3)

alas macroquad was too good to be true. 
their build system sucks ass if you want to use any other wasm library in the universe. 
it's a little sad that i was tempted to do a similar thing because I also dislike wasm-bindgen but this is such a pain. 

First attempt messing around 
```html
<canvas id="glcanvas" tabindex='1'></canvas>
<script src="./quad.js"></script>
<script type="module">
    // removed import from env in
    // REMOVE imports['env'] =___dasldas; in __wbg_get_imports
    // change default export to __wbg_get_imports
    // load("linrays_sb3.wasm");
    import __wbg_get_imports from "./linrays_sb3.js";
    let wasm_bg = await __wbg_get_imports();
    console.log(wasm_bg);
    let macroquad = window.importObject;
    console.log(macroquad);
    macroquad.env.wbg = {...wasm_bg.wbg, ...macroquad.env.wbg};
    console.log(macroquad);
    load("linrays_sb3_bg.wasm");
    // init();
</script>
```

i was pretty close actually, found someone else's solution:
- https://github.com/not-fl3/macroquad/issues/212
- https://github.com/not-fl3/miniquad/wiki/JavaScript-interop
- https://gist.github.com/profan/f38c9a2f3d8c410ad76b493f76977afe

the thing where browser console log of an object doesnt make a copy, it just gives you live updating view is such a foot gun. 
very glad i randomly read someone complaining about it or id be so confused for so much longer. 

This does hurt my goal of it just being a normal rust project tho. 

## adding async to the compiler (Jan 3)

Note: everywhere here `async` refers to my shitty runtime not the normal rust futures system. 

before messing with stuff I want to do a bit of cleanup. 
instead of passing around strings of rust src, always combine with the type since we know when the expression is emitted. 

For doing async i want a bit more structure than an emitted block just being an opaque string of src code. 
Need to know if something is a sync stmt or an IoAction or a FutFn. 
Sync code is faster so whenever possible want to collapse blocks into one sync block and not have the FutFn closure around it.

The custom functions being sync or async will create similar type checking problem where need to look at everything 
before you can know what's async because it calls some other async. 

Using `this: &mut Self` instead of `&mut Self` in sync fns means I don't need to switch between self and this based on fn colour when generating code. 
The async ones have it passed in and self is a magic keyword you cant assign to. 
wierd that that's different, cant call like method if first param is your type instead of magic self, need to assign in body like in async fns which looks a bit silly. 

Was planning on having the ScratchProject register as either sync or async 
and then only impl that method on sprites and world has to comptime switch and call the right one. 
But that seems like silly extra work because sync projects are an edge case (non-interactive) 
and the overhead of a single async dispatch wrapping all your sync code is negligible. 
Still want the user code to not have to write async fns if all are sync. 

Trying to have default trait method forward async messages to the sync future. 
The trait (or method) can't require Sized because the whole point is to use trait objects.
So then the Self is ?Sized. 
But I don't understand why all the methods in Any require Sized. 
why might vtable kinds not match?
```rust
// misleading name
fn is_any_unsized<T: Any + ?Sized>(this: &mut dyn Any) -> bool {
    let t = TypeId::of::<T>();
    let concrete = this.type_id();
    t == concrete
}
pub fn trusted_cast<'a, O: Sprite<S, R> + ?Sized>(&self, sprite: &'a mut dyn Any) -> &'a mut O {
    if is_any_unsized::<O>(sprite) {
        unsafe { &mut *(sprite as *mut dyn Any as *mut O) }
    } else {
        panic!()
    }
}
```
I must have some dumb misinterpretation?  
it works if you wrap it, so you can require Sized and then have the compiler generate the impl with one fn call. 
```rust
pub fn forward_to_sync<S: ScratchProgram<R>, R: RenderBackend<S>, O: Sprite<S, R> + Sized>(msg: Trigger<S::Msg>) -> Box<FnFut<S, R>> {
    Box::new(move |ctx, this| {
        let this: &mut O = ctx.trusted_cast(this);
        this.receive(ctx, msg);
        IoAction::None.done()
    })
}
```

Struggle with typeid not matching and can't figure out how to get a meaningful type_name out of it. 
Maybe you can't without comptime knowing the type cause then it would need to include metadata for literally everything. 
Trying to call it on `&mut self.custom[c.owner]` but that's a `&mut Box<dyn Sprite>` so it the Any is the box, 
need to call it on `&mut *self.custom[c.owner]`. Only works using nightly compiler for trait_upcasting. 
I don't want to make everything in the world an Any cause then idk how you'd call things. 

Need to figure out how to poll stdin without blocking. Do I really need to spawn a new thread? 
I did it for my asm snake a while ago but that also involved turning on raw mode for game input so maybe won't work without that, i dont remember. 

It's funny how deranged-ly commented the code is when I'm less confident in what im doing. 

## thinking about async (Jan 2)

i wonder if i could pass around the mutable stuff through the context when polling a future. 
So i can set it up that i call poll on a future made by the compiler from an async function 
and it passes in a waker thingy that i define but still have the problem 
that it closes over the mut reference arguments to the function. 
what i want is to get those from the waker every time i poll the future? 
im not sure if that makes sense. 
i can implement poll myself on a struct and in there get interesting info out of my context. 

I was thinking of my own ScratchFn as a trait but really what I'm doing is reimplementing closures. 
so what if i just emit FnMuts. and then its like old javascript where you just did promises as callbacks. 
and its like fine because the compiler's generating it so the pain of writing infinitely nested things doesn't matter .

I guess the downside of using closures instead of generating the struct myself is each await point is its own Box, 
but most of them don't capture anything (only loops close over their counter) so most don't actually allocate. 

## macroquad backend (Jan 2)

Need a little wrapper of main because they use async next frame for easier wasm but I don't want to impose that on all backends. 

- save project.json file for debugging panics
- treat looks_costume as string for now
- macroquad sprite rendering & stamping
- test program that just stamps imgs in a pattern to compare positioning to real scratch

TODO: support ios and android, should be easy with some backend. 
TODO: cli flags to easily build native/wasm/phone. readme note that you don't need to use them

## idea: scratch blocks bijection

Convert to/from the syntax of https://github.com/scratchblocks/scratchblocks  
There's a converter but only for scratch 2. 

I'd make it a dumb transpiler with a one-to-one correspondence with scratch projects. 
No interesting optimisation or usability features. 

The motivation for me would be then my web demo could also generate that and use their thing to render it. 
I don't care about having an editor, but it would be cool to see what's being compiled in a readable form.
So I guess I don't actually need to parse it, I could just add it as a backend. 
Would also be cool to add it to [Johan-Mi/sb3-builder](https://github.com/Johan-Mi/sb3-builder) so anything using that could get it for free. 
Then I could use that and re-output sb3 after ast parsing stage. 
Which isn't strictly useful, but I find making a full loop appealing as a way to sanity check that I didn't mess something up. 
Like I should be able to parse and output something that scratch can import, and then you know I didn't lose any information. 

## idea: canvas? (Jan 1)

```rust 
//! A backend that uses the simple HTML5/JS canvas api directly (no wasm-bindgen/emscripten and no webgl/webgpu).
#[cfg(not(target_arch = "wasm32"))]
compile_error!("The canvas backend only supports wasm.");
extern "C" {
  pub fn draw_line(c: usize, x1: f64, y1: f64, x2: f64, y2: f64, size: f64, r: u8, g: u8, b: u8);
  pub fn draw_pixel(c: usize, x: f64, y: f64, r: u8, g: u8, b: u8);
  pub fn draw_text(c: usize, x: f64, y: f64, chars: *const u8, len: usize);
  pub fn draw_texture(c: usize, t: usize/* TextureId */, x: f64, y: f64, w: f64, h: f64);
  pub fn load_texture(c: usize, bytes: *const u8, len: usize) -> usize/* TextureId */;
  pub fn fetch_texture(c: usize, path: *const u8, len: usize) -> usize/* TextureId */;
}
```


## including assets (Jan 1)

As practice, make line drawing go through render handle so don't need to save them all in a list. 
TODO: make sure the pixel set check actually makes it faster. 
wasted so much type trying to get lifetimes to work out on the softbuffer backend that i don't even care about. 

Want the option to include in the binary or download at runtime (need some sort of caching I guess).

Backend probably just wants a list of textures so when a sprite wants to switch texture it needs to look it up by name 
or scratch index (since you can do next_costume) so need a different mapping of str/int -> int for each type of sprite. 

TODO: figure out how to use notan's draw onto an offscreen texture and render that every frame instead 
of saving all the pen stamps in a list and replaying them. Needs to be the same texture used for pen pixels. 
Logic being sprites are draw in their current position each frame so need to clear the screen but pen persists until explicitly cleared. 

TODO: unrelated, I wonder if scratch-compiler-2 return values support recursive fibonacci

was very distressed that the softbuffer one was calling pen_line but notan one wasn't. 
was because softbuffer was sending flag event on redraw requested, so it happened twice (init + resize?) 
so it had to move from end pos to start pos while notan was just doing it once on init. 

## nicer interface (Dec 31)

Want to add the ability to just load a project from scratch by url to my demo site. 

the scratch api for getting the project token doesn't let you do cors stuff but the turbowarp one does.
very friendly of them to not make everyone else set up their own lambda function or whatever to circumvent. 
and the scratch endpoint for actually getting the project.json doesn't have the cors thing tho which is odd (that's what turbowarp uses).

```js
let scratch = async (id) => {
  let token = (await (await fetch(`https://trampoline.turbowarp.org/api/projects/${id}`)).json()).project_token;
  return (await fetch(`https://projects.scratch.mit.edu/${id}?token=${token}`)).json();
}
console.log(await scratch("396320314"));
```

anyway that works in the console from my site.

## notan backend (Dec 31)

- https://github.com/Nazariglez/notan
- https://nazariglez.github.io/notan-web/

Randomly chosen library that claims to make it easier than directly interacting with the cpu crates. 
Should eventually do my own wgpu backend but don't care right now. 
I had an unpleasant previous experience with nannou making every rectangle draw in their pretty 
builder api take like several hashmap lookups that were notably slow, so I'm a little suspicious 
of friendly stuff but let's see.
It's pretty chonky to build so my not sharing workspace for generated projects is annoying. 

god-damn its annoying that I cant just pass the world into the init function because they want to 
make stupid pretty builder for the easy case now you cant do anything. 

Current runtime sizes on trivial mandelbrot so all size is renderer with (release, panic=abort, strip=debuginfo, lto=true):   
native: (softbuffer=672 KB, notan=1829 KB) wasm: (notan= 985 KB + 55 KB js)
pleasing that trunk serve just works. 

TODO: notan has nice egui stuff. should make a variable view window. 

## planned refactoring (Dec 31)

(1) TODO:
I think I should separate the simulation world that owns the sprites and globals from the
driver that runs the event loop. Maybe all the sprite methods become fn (ctx: Ctx<Self>, ...args)
where Ctx { &mut vars, &mut sprite, &mut globals, &mut BackendFrameCtx }
and then you call builtins on that context and its able to do things like render immediately instead of buffering your lines.
and that would permit different rendering backends in a less painful way.
main would become something like BackendImpl::run(World::new(..))
and you want as much logic as possible to be in the world and the backend to just
emit input events and accept render commands .
need to do it bit by bit, so it always compiles throughout the process or im gonna get bored of it when it sucks,
but the end goal is being able to drop in a wasm/canvas backend as well.

TODO: 
I should also think about splitting up the compiler backend more because currently the rust thing 
is doing a lot of type coercing work that would need to be replicated if I wanted to add a c one for example. 
Maybe a new pass that adds explicit cast expressions and later that can also split functions across await points. 

TODO: 
Need to have a nice CLI for the compiler and try to make a sane makefile for all my tests. 
Currently have to change so many places to add one. Can just use clap or whatever but should also have 
a serde for that struct so can get it from a string in the web one or have a cargo.toml equivalent 
for a real project instead of being forced to do your build config in cli arguments. tho why not allow that 
too for projects like this that have lots of targets. Want to make switching backends as easy as possible 
for users: would be cool if you don't need to rerun my compiler, and it was just a feature flag on the 
generated project since they all expose the same interface to the scratch code. 
Would be nice to auto-run the tests that just generate one frame and put them all in an image, so 
I can see all results at once. Is it ugly to commit that, so you can see the diff?

TODO:
Also want to add a lib target to scratch-compiler, so I can include examples in my web demo without 
needing to ship giant json blobs. Need to add a feature flag to turn off cranelift backend cause while 
it's very cool, I don't need it in that context. Might be a pain to rework file system dependency.

(1) DONE: the refactor to make runtime backend a trait.
Couldn't put the user sprite in the Ctx struct because then it would have to be generic over it but 
the world only has dyn trait objects so instead user code is still methods and ctx just has everything else. 
But the main point was to be a place to hook in the render backend, so it's fine. 

TODO: 
Maybe the thing contained in the Ctx shouldn't be the backend struct itself but some sort of handle 
to the current frame? Is that what associated types on traits are or are they like iterators where 
you have to specify? Oh, it's both. Like Iterator could be any but Zip doesn't make you re-state the 
associated types of A and B. So that should work and the backend could choose itself as the type if that makes sense. 
Had to think about it but difference of associated vs generics is that there can only be one so caller doesn't specify.
- https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#specifying-placeholder-types-in-trait-definitions-with-associated-types

## Great Success (Dec 30)

Pretty sure I decided all strings coerce to zero because `"1"` did but that's probably actually the string with the quotes.
I can have optimisation that recognises the idiom of `(x == (0 + x)) === x.is_num()` and just call a runtime method.
It's cool that I can emit code that shows intent more clearly (as long as there's no possible other interpretation for that expression).
For now, it's a hack because I only recognise that specific shape of tree and miss-compile if I don't notice.
TODO: test flag that enables and disables opts like that, so I can have little sanity checks that they behave the same.

TODO: test flag that reverses iteration order.

wrote little scratch programs that asserts sanity checks.
very pleasing to be able to run it in a second and make sure i didn't make any dumb mistakes in simple expressions. 

I don't quite feel prepared for doing it in the tres language yet but let's do a scratch one as a start.
Mistakes I made: missing else branch on if, how do you have a unit statement?
oh then & else were macros, and you need `do` to create a block with multiple things
TODO: can I make their compiler emit the stacks not all over-lapping each-other? did they manually rearrange for their shared scratch versions? 

TODO: write a note in the readme about using turbo mode and run without screen refresh
with all logic in a custom block for fair comparisons because i almost thought mine was way better
than it actually is because i havent implemented yielding yet, so it cheats. 

Update: [they took my fix](https://github.com/Johan-Mi/scratch-compiler/pull/12)! very pleasing :)

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

NOTE: profiler seems to hang if you start try to spawn a new process (like when my code wants to run cargo check) ???

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
