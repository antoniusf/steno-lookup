<script>
    import { keylistToStroke, strokeToText, textToStroke, strokeToKeydict } from './util.js';
    import { createEventDispatcher } from 'svelte';
    
    const keys = ["#", "S-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "*", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z"];
    const replace_map = {"number": "#", "star": "*"};
    
    let state = {};
    for (const key of keys) {
	state[key] = false;
    }

    export let stroke = 0;

    const dispatch = createEventDispatcher();

    //$: stroke = keylistToStroke(
    //    Object.entries(state)
    //	    .filter(([key, state]) => state) // use only keys where state is true
    //	    .map(([key, state]) => (key == "star")? "*" : key) // replace "star" with "*"
    //   );
    $: state = strokeToKeydict(stroke);

    let active_touches = {};

    function touch_start(event) {
	console.log(event.touches.length);
	for (let i = 0; i < event.touches.length; i++) {
	    let touch = event.touches[i];
	    if (!active_touches.hasOwnProperty(touch.identifier)){
		let button = replace_map[event.target.id] || event.target.id;
		state[button] ^= true;
		let mode = state[button];
		active_touches[touch.identifier] = { x: touch.clientX, y: touch.clientY, mode: mode };
	    }
	}
	stroke_changed();
    }

    function touch_move(event) {
	let touches = event.changedTouches;
	let changed = false;
	for (let i = 0; i < touches.length; i++) {
	    let touch = touches[i];
	    let old_touch = active_touches[touch.identifier];
	    if (!old_touch) {
		continue;
	    }

	    // estimate width of a button by window width
	    // there's ten buttons, but i'm including a bit of extra buffer
	    let button_width = window.innerWidth / 20;
	    let delta_x = touch.clientX - old_touch.x;
	    let delta_y = touch.clientY - old_touch.y;
	    let delta = Math.sqrt(delta_x ** 2 + delta_y ** 2);

	    let move_x = delta_x / delta * button_width;
	    let move_y = delta_y / delta * button_width;

	    let step = 0;
	    while (delta > 0) {
		// draw a straight line between both points and then subdivide it
		// this way we'll make sure that we hit all in-between buttons
		// we're also going backwards, since (old_touch.x, old_touch.y) will have
		// already been handled
		let x = touch.clientX - move_x * step;
		let y = touch.clientY - move_y * step;
		let target = document.elementFromPoint(x, y);
		let button = replace_map[target.id] || target.id;
		if (state.hasOwnProperty(button)) {
		    if (state[button] != old_touch.mode) {
			state[button] = old_touch.mode;
			changed = true;
		    }
		}
		delta -= button_width;
		step += 1;
	    }

	    old_touch.x = touch.clientX;
	    old_touch.y = touch.clientY;
	}

	if (changed) {
	    stroke_changed();
	}
    }

    function touch_end(event) {
	let touches = event.changedTouches;
	for (let i = 0; i < touches.length; i++) {
	    delete active_touches[touches[i].identifier];
	}
    }

    function stroke_changed() {
	dispatch('strokeChanged', {
	    stroke: keylistToStroke(
		Object.entries(state)
		    .filter(([key, state]) => state) // use only keys where state is true
		    .map(([key, state]) => key)
	    )
	});
    }

    function handleClick(event) {
	// toggle the button's state
	const button = replace_map[event.target.id] || event.target.id;
	state[button] ^= true;

	dispatch('strokeChanged', {
	    stroke: keylistToStroke(
		Object.entries(state)
    		    .filter(([key, state]) => state) // use only keys where state is true
                    .map(([key, state]) => key)
	    )
	});
    }
</script>

<style>
  div#steno-keyboard {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    grid-template-rows: auto auto auto auto;
    grid-template-areas:
      "number number number number number number number number number number"
      "S- T- P- H- star -F -P -L -T -D"
      "S- K- W- R- star -R -B -G -S -Z"
      ".  .  A- O-   .  -E -U  .  .  .";
    grid-column-gap: 0.9%;
    grid-row-gap: 2.5%;
    max-width: 70em;
    width: calc(100% - 2em);
    margin: 1em 1em 3em;
  }
  
  button {
    width: 100%;
    padding: 0;
    margin: 0;
    padding-bottom: 120%;
    background-color: #aaa;
    border: none;
    color: white;
    cursor: pointer;
    height: 0;
  }

  button#number {
    grid-area: number;
    padding-bottom: 2%;
  }

  button.active {
    background-color: #000;
  }
  
  button.top-row {
    border-radius: 3% / 2%;
  }
  
  button.bottom-row,
  button.right-vowel,
  button.left-vowel {
    border-radius: 3% / 2%;
    border-bottom-left-radius: 50% 20%;
    border-bottom-right-radius: 50% 20%;
  }
  
  button.long-key {
    border-radius: 3% / 1%;
    border-bottom-left-radius: 50% 10%;
    border-bottom-right-radius: 50% 10%;
    height: 100%;
    padding: 0;
  }
  
  div.vowel-container {
    width: 100%;
    height: auto;
    padding: 0;
    margin: 0;
  }
  
  button.left-vowel {
    position: relative;
    left: 25%;
    top: 30%;
    vertical-align: text-bottom;
  }
  
  button.right-vowel {
    position: relative;
    left: -25%;
    top: 30%;
    vertical-align: text-bottom;
  }

  button#S- {
    grid-area: S-;
  }

  button#T- {
    grid-area: T-;
  }

  button#K- {
    grid-area: K-;
  }

  button#P- {
    grid-area: P-;
  }

  button#W- {
    grid-area: W-;
  }

  button#H- {
    grid-area: H-;
  }

  button#R- {
    grid-area: R-;
  }

  div#A- {
    grid-area: A-;
  }

  div#O- {
    grid-area: O-;
  }

  button#star {
    grid-area: star;
  }

  div#-E {
    grid-area: -E;
  }

  div#-U {
    grid-area: -U;
  }

  button#-F {
    grid-area: -F;
  }

  button#-R {
    grid-area: -R;
  }

  button#-P {
    grid-area: -P;
  }

  button#-B {
    grid-area: -B;
  }

  button#-L {
    grid-area: -L;
  }

  button#-G {
    grid-area: -G;
  }

  button#-T {
    grid-area: -T;
  }

  button#-S {
    grid-area: -S;
  }

  button#-D {
    grid-area: -D;
  }

  button#-Z {
    grid-area: -Z;
  }
</style>

<div id="steno-keyboard">
  <button id="number" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["#"]}></button>
  <button id="S-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["S-"]} class="long-key"></button>
  <button id="T-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["T-"]} class="top-row"></button>
  <button id="K-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["K-"]} class="bottom-row"></button>
  <button id="P-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["P-"]} class="top-row"></button>
  <button id="W-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["W-"]} class="bottom-row"></button>
  <button id="H-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["H-"]} class="top-row"></button>
  <button id="R-" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["R-"]} class="bottom-row"></button>
  <div id="A-" class="vowel-container">
    <!--note: the button ids don't have the dash, to distinguish them from the div ids!
              (and also to make interfacing with the stroke library easier, where the vowels
               don't have dashes either)
      -->
    <button id="A" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["A"]} class="left-vowel"></button>
  </div>
  <div id="O-" class="vowel-container">
    <button id="O" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["O"]} class="left-vowel"></button>
  </div>
  <button id="star" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["*"]} class="long-key"></button>
  <div id="-E" class="vowel-container">
    <button id="E" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["E"]} class="right-vowel"></button>
  </div>
  <div id="-U" class="vowel-container">
    <button id="U" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["U"]} class="right-vowel"></button>
  </div>
  <button id="-F" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-F"]} class="top-row"></button>
  <button id="-R" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-R"]} class="bottom-row"></button>
  <button id="-P" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-P"]} class="top-row"></button>
  <button id="-B" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-B"]} class="bottom-row"></button>
  <button id="-L" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-L"]} class="top-row"></button>
  <button id="-G" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-G"]} class="bottom-row"></button>
  <button id="-T" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-T"]} class="top-row"></button>
  <button id="-S" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-S"]} class="bottom-row"></button>
  <button id="-D" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-D"]} class="top-row"></button>
  <button id="-Z" on:touchstart|preventDefault={touch_start} on:touchmove={touch_move} on:touchend={touch_end} on:touchcancel={touch_end} class:active={state["-Z"]} class="bottom-row"></button>
</div>
