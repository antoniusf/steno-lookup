# a small steno lookup app

this is a small and lightweight app i made that lets you look up definitions from your steno dictionary on the go. it uses service workers to provide offline support, and webassembly to make lookups fast while keeping the memory footprint as small as possible. the ui framework is svelte, which is why the ui is so lightweight.

### things that are nice

- small (less than 100kB for the whole app)
- does not need the internet at all after it's been loaded the first time (only if you want to update)
- fast (runs well on my 8-year-old galaxy s2)
- low memory usage, so the tab can stay open in the background (just a few megabytes, about as much as the dictionary in json form)

### things that are not so nice

- the current lookup/dictionary handling code is kind of complex and not that easy to modify. i think part of that is the price to pay for the performance characteristics, but it might be worth trying alternatives and seeing if we can do without it.
- absolutely does not work without a modern browser, since webassembly support is crucial. also, i never want to do css layout with anything other than grid/flexbox ever again.

### screenshots

![screenshot of the app in lookup mode. there is a title, two buttons for switching to the other modes, a text entry box with the word "test" entered, and a list of results that shows definitions translating to "test".](screenshot-lookup.png) ![screenshot of the app in find stroke mode. below the title and mode buttons is a schema of a steno keyboard, with some keys highlighted. there is also a text entry box with the characters "T*ES", corresponding to the stroke shown by the keyboard. below that is a single result, showing that "T*ES" translates to "test".](screenshot-find-stroke.jpg) ![screenshot of the dictionary load screen. below the header is a box showing the currently loaded dictionary, with the option to remove it. below that is a subheading saying "Updates", some text saying that an update is available, and a button saying "Update!".](screenshot-load.png)

## development setup

you'll need [node](https://nodejs.org) for the main app, and [rust](https://www.rust-lang.org/) for the wasm module. optionally, you can also install wasm-strip from the [webassembly binary toolkit](https://github.com/WebAssembly/wabt) to make the produced wasm smaller.

the main app is based on the project template for [svelte](https://svelte.dev) apps from https://github.com/sveltejs/template. i've included the relevant parts from their readme below.

### Get started

Install the dependencies...

```bash
cd svelte-app
npm install
```

...then start [Rollup](https://rollupjs.org):

```bash
npm run dev
```

Navigate to [localhost:5000](http://localhost:5000). You should see your app running. Edit a component file in `src`, save it, and reload the page to see your changes.

By default, the server will only respond to requests from localhost. To allow connections from other computers, edit the `sirv` commands in package.json to include the option `--host 0.0.0.0`.


### Building and running in production mode

To create an optimised version of the app:

```bash
npm run build
```

You can run the newly built app with `npm run start`. This uses [sirv](https://github.com/lukeed/sirv), which is included in your package.json's `dependencies` so that the app will work when you deploy to platforms like [Heroku](https://heroku.com).

### building the rust code

the rust code lives in the `wasm` directory. it does not rely on `wasm-pack`, since that includes a lot of boilerplate code and requires an allocator, both of which i've tried to avoid in the current version. instead, it just builds directly to the `wasm32-unknown-unknown` target, which is suprisingly simple. however, you may have to install the target manually with `rustup target add wasm32-unknown-unknown`.

once you have done this, switch to the wasm directory and just run `cargo build --release`. once that's done, run the `post-build.sh` script. (if you're on linux – i'm sorry that this doesn't work cross platform currently.) this script does two things:

1. run `wasm-strip` on the binary, to remove the debug sections produced by the rust compiler. (yes, it does this even in release mode. no, i did not figure out how to turn this off.)
2. copies the file from `target/wasm32-unknown-unknown/release/wasm.wasm` into the `public` directory of the web app, renaming it to `helpers.wasm`.

you can do these steps manually if you can't run the script. running `wasm-strip` is optional, too. you really only need to copy the file to the right place.