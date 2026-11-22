Below is the conceptual map I’d use. Treat it as a tutorial, not an implementation. I have not changed files.

## 0. Two corrections to the earlier sketch

1. In `xkbcommon 0.9.0`, the type is `xkb::StateComponent`, not `xkb::StateFlags`.  
   `xkb::compose::StateFlags` exists, but it is only for `compose::State::new`.

2. `xkb::Keymap::new_from_names` takes `options: Option<String>`, not `Option<&str>`.  
   That matters if you later store options as `Arc<str>` in a config snapshot.

So this:

```rust
const MOD_MASK: xkbcommon::xkb::StateFlags = ...
```

should be:

```rust
const WAYLAND_MODS_CHANGED: xkb::StateComponent = ...
```

---

# 1. The xkb object model

| Thing | What it is | Mutable? | Purpose |
|---|---|---:|---|
| `xkb::Context` | compiler environment: include paths, atom table, config defaults | no | Needed to create keymaps and compose tables |
| `xkb::KeymapSpec` / your config value | RMLVO inputs: rules/model/layout/variant/options | yes, in config | Describes *what keymap you want* |
| `xkb::Keymap` | compiled keyboard description | no | Keycodes → keysyms, modifiers, layouts, repeat |
| `xkb::State` | current pressed/latched/locked state for one keyboard | yes | Computes keysyms and tracks modifier changes |
| `xkb::compose::Table` | locale compose sequence database | no | Dead keys / Compose key sequences |
| `xkb::compose::State` | per-device compose sequence cursor | yes | Tracks “dead accent waiting for next key” |
| `ModMask` | `u32` bitmask of actual modifiers | value | `Shift`, `Ctrl`, `Caps`, etc. as bits |
| `StateComponent` | `u32` bitmask of *categories that changed* | value | `MODS_DEPRESSED`, `LAYOUT_EFFECTIVE`, etc. |
| `Modifiers` | your struct with 4 protocol masks | value | Exactly what `wl_keyboard.modifiers` needs |

Important distinction:

```rust
xkb::ModMask       // “which modifiers are active”
xkb::StateComponent // “which parts of xkb state changed”
```

They are both `u32`, but semantically different.

---

# 2. What is `ModMask`?

`xkb::ModMask` is a bitmask where each bit corresponds to a modifier index in the keymap.

Example, conceptually:

```text
bit 0 -> Shift
bit 1 -> Lock/Caps
bit 2 -> Control
bit 3 -> Mod1/Alt
...
```

So a depressed mask of `0b0001` means “Shift is physically down”.

Wayland does **not** send modifier names. It sends raw masks:

```text
wl_keyboard.modifiers(
    mods_depressed,
    mods_latched,
    mods_locked,
    group
)
```

The client uses its own xkb state plus your keymap to interpret those bits.

That is why your `Modifiers` struct exists:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub depressed: xkb::ModMask,
    pub latched: xkb::ModMask,
    pub locked: xkb::ModMask,
    pub group: xkb::LayoutIndex,
}
```

That is not config metadata. It is protocol state.

---

# 3. What is `MOD_MASK` / `WAYLAND_MODS_CHANGED`?

Your constant is not a modifier mask. It is a `StateComponent` mask: “which kinds of xkb state changes require us to send `wl_keyboard.modifiers`?”

I would name it like this:

```rust
const WAYLAND_MODS_CHANGED: xkb::StateComponent =
    xkb::STATE_MODS_DEPRESSED
    | xkb::STATE_MODS_LATCHED
    | xkb::STATE_MODS_LOCKED
    | xkb::STATE_LAYOUT_EFFECTIVE;
```

Meaning:

- `STATE_MODS_DEPRESSED`: a physical modifier was pressed/released
- `STATE_MODS_LATCHED`: sticky/latched modifier changed
- `STATE_MODS_LOCKED`: Caps Lock-like modifier changed
- `STATE_LAYOUT_EFFECTIVE`: active keyboard group/layout changed

Those correspond to the fields Wayland clients need.

`update_key()` returns a `StateComponent`. You check:

```rust
if changed & WAYLAND_MODS_CHANGED != 0 {
    // send wl_keyboard.modifiers after the key event
}
```

Including `STATE_MODS_EFFECTIVE` is usually redundant because effective state is derived from depressed/latched/locked. The important thing is that you serialize the three separate components Wayland wants.

---

# 4. What is `KeymapSpec`?

`KeymapSpec` is a pure description of the desired keymap. It is the RMLVO input:

```text
Rules
Model
Layout
Variant
Options
```

Example:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeymapSpec {
    pub rules: Arc<str>,      // "evdev"
    pub model: Arc<str>,      // "pc105"
    pub layout: Arc<str>,     // "us"
    pub variant: Arc<str>,    // ""
    pub options: Option<Arc<str>>, // None = default, Some("") = no options
}
```

`Option<Arc<str>>` matters because in xkbcommon:

- `None` means “use default options”
- `Some("")` means “explicitly no options”

Those are different.

`KeymapSpec` is not xkb state. It is closer to a config value. It belongs to your policy/config layer.

---

# 5. Why do you have a `FxHashMap<KeymapSpec, xkb::Keymap>`?

Because compiling an xkb keymap is relatively expensive, and many devices can share the same layout.

If you have 5 keyboards all using `us`, you do not want to compile the same keymap 5 times.

`xkb::Keymap::clone()` is cheap because it calls `xkb_keymap_ref`, an atomic refcount increment.

So the cache does this:

```rust
fn get_or_compile(&mut self, spec: &KeymapSpec) -> Result<xkb::Keymap, SeatError> {
    if let Some(map) = self.keymaps.get(spec) {
        return Ok(map.clone()); // refcount bump
    }

    let options = spec.options.as_ref().map(|s| s.to_string());

    let map = xkb::Keymap::new_from_names(
        &self.context,
        spec.rules.as_ref(),
        spec.model.as_ref(),
        spec.layout.as_ref(),
        spec.variant.as_ref(),
        options,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or(SeatError::Keymap)?;

    self.keymaps.insert(spec.clone(), map.clone());
    Ok(map)
}
```

This runs on device add or config reload, not on every key event.

`FxHashMap` is fine, but this path is cold enough that `std::collections::HashMap` would also be fine. Do not over-optimize here.

---

# 6. Can you update keymap fields individually?

No, not really.

There are two different kinds of “fields”:

## Keymap description fields

`layout`, `variant`, `options`, etc. are compiler inputs.

You do not mutate an `xkb::Keymap`. It is immutable compiled data. If the user changes layout from `us` to `de`, you compile a new keymap and create a new `xkb::State` from it.

So:

```text
config change -> new KeymapSpec -> compile/get cached Keymap -> recreate State
```

## Runtime state fields

Modifiers, layout group, latched state, pressed keys — those live in `xkb::State`.

You do not assign them directly either. Their fields are private and xkb must keep derived state consistent. You update them with:

```rust
state.update_key(keycode, direction)
```

or in special cases `update_mask(...)`.

For a compositor, `update_key()` is the normal path.

---

# 7. Better abstraction for keymaps

The earlier `KeyboardState::new(&Keyboards)` design is a bit tangled because the child reaches back into the parent.

Cleaner split:

```rust
struct KeymapCache {
    context: xkb::Context,
    maps: FxHashMap<KeymapSpec, xkb::Keymap>,
}

impl KeymapCache {
    fn new() -> Self { ... }

    fn get_or_compile(&mut self, spec: &KeymapSpec)
        -> Result<xkb::Keymap, SeatError>
    { ... }
}

struct KeyboardState {
    state: xkb::State,
    keymap: xkb::Keymap,
    compose_state: Option<xkb::compose::State>,
}

struct KeyboardPool {
    keymaps: KeymapCache,
    compose_table: Option<xkb::compose::Table>,
    devices: SecondaryMap<DeviceKey, KeyboardState>,
}
```

Then `KeyboardPool::add_device` does:

```rust
let keymap = self.keymaps.get_or_compile(spec)?;
let compose_state = self.compose_table.as_ref().map(|table| {
    xkb::compose::State::new(table, xkb::compose::STATE_NO_FLAGS)
});

let state = KeyboardState {
    state: xkb::State::new(&keymap),
    keymap,
    compose_state,
};

self.devices.insert(key, state);
```

Why keep `keymap` inside `KeyboardState` even though `xkb::State` internally references it?

Because you need it for things like:

```rust
self.keymap.key_repeats(keycode)
```

`State::get_keymap()` exists, but it returns a cloned/refcounted keymap, so calling it on every key event adds needless refcount churn. Holding the cheap clone directly is better.

---

# 8. What is `compose::Table` and why is it separate from `Context`?

`xkb::Context` is the general xkb compiler environment.

`xkb::compose::Table` is a locale-specific database of compose sequences, for example:

```text
dead_acute + e -> é
Multi_key + a + a -> å
```

These are not physical key mappings. They are textual sequences.

You need:

```text
Context       -> required to compile keymaps and compose tables
Keymap        -> physical key + mods -> keysym
Compose Table -> sequence of keysyms -> resulting keysym
Compose State -> per-device sequence progress
```

The compose table is shared and immutable. The compose state is per keyboard because each keyboard can be in the middle of a different compose sequence.

Do you need compose right now?

Probably not.

For plain Wayland `wl_keyboard`, clients do their own compose handling. Wayland sends keycodes and keymap, not composed text.

You only need compose if you want compositor-side text keysyms for:

- Lua binds involving composed symbols
- text input features
- dead-key handling before passing something to a text protocol

If you do not need that yet, drop `compose_table`, `compose_state`, and `composed` from the first implementation. Add them later.

---

# 9. `process_key` line by line

Here is the cleaned-up version I would reason about:

```rust
fn process_key(
    &mut self,
    evdev_keycode: u32,
    pressed: bool,
) -> TranslatedKey {
    let keycode = xkb::Keycode::new(evdev_keycode + 8);

    let keysym = self.state.key_get_one_sym(keycode);

    let repeats = pressed && self.keymap.key_repeats(keycode);

    let direction = if pressed {
        xkb::KeyDirection::Down
    } else {
        xkb::KeyDirection::Up
    };

    let changed = self.state.update_key(keycode, direction);

    let modifiers = if changed & WAYLAND_MODS_CHANGED != 0 {
        Some(self.serialized_mods())
    } else {
        None
    };

    let composed = self.feed_compose(keysym, pressed);

    TranslatedKey {
        keysym,
        composed,
        repeats,
        modifiers,
    }
}
```

Line by line:

## `let keycode = xkb::Keycode::new(evdev_keycode + 8);`

libinput/evdev keycodes are 8 lower than xkb keycodes.

So:

```text
evdev 30 -> xkb 38
```

That `+ 8` is required for every xkb call.

## `let keysym = self.state.key_get_one_sym(keycode);`

Ask xkb: “given my current state, what symbol does this physical key produce?”

This is done **before** `update_key`, which is the conventional order.

Why before?

If you press Shift itself, you want the event to say `Shift_L`, not the shifted meaning of Shift. Also if you release a modifier, the keysym should be computed while that modifier was still logically active.

For binds, this is the usual behavior.

## `let repeats = pressed && self.keymap.key_repeats(keycode);`

Ask the keymap whether this key is eligible for autorepeat.

Only meaningful on press. A released key never repeats.

If you do not implement key repeat yet, remove this field entirely.

## `let direction = ...`

Convert your `bool` into xkb’s `KeyDirection` enum.

```text
pressed == true  -> Down
pressed == false -> Up
```

## `let changed = self.state.update_key(keycode, direction);`

This mutates the xkb state.

For a modifier key press, it updates depressed modifiers. For a release, it clears them. It also updates latched/locked layout state and LEDs according to the keymap rules.

Important: calls must be balanced. Every `Down` should eventually have an `Up`, or you get stuck modifiers.

## `let modifiers = if changed & WAYLAND_MODS_CHANGED != 0 { ... }`

Only serialize modifiers when something Wayland cares about changed.

This avoids 4 xkb FFI calls on every ordinary key event.

For normal key presses like `a`, `changed` is usually `0`, so no `wl_keyboard.modifiers` event is needed.

## `Some(self.serialized_mods())`

When modifiers did change, produce the four masks Wayland needs.

## `self.feed_compose(keysym, pressed)`

Optional.

Only press events feed compose sequences. If a compose sequence completes, you may get a composed keysym such as `é`.

Do not put that in `wl_keyboard.key`. Wayland key events remain raw keycode + pressed/released. Composed output belongs in a separate text/bind abstraction.

## `TranslatedKey { ... }`

Return only the translation result. The seat adds:

- `DeviceKey`
- timestamp
- final seat-wide modifier state
- protocol event type

---

# 10. What is `serialized_mods`?

“Serialized” here does not mean JSON or byte serialization. In xkb terminology it means “convert internal state into the compact mask representation used by clients.”

```rust
fn serialized_mods(&self) -> Modifiers {
    Modifiers {
        depressed: self.state.serialize_mods(xkb::STATE_MODS_DEPRESSED),
        latched: self.state.serialize_mods(xkb::STATE_MODS_LATCHED),
        locked: self.state.serialize_mods(xkb::STATE_MODS_LOCKED),
        group: self.state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE),
    }
}
```

Meaning:

- `depressed`: physically held modifiers
- `latched`: one-shot modifiers
- `locked`: toggled modifiers
- `group`: effective layout index

That maps directly to Wayland:

```rust
wl_keyboard::modifiers(
    mods.depressed,
    mods.latched,
    mods.locked,
    mods.group,
)
```

Do not use only:

```rust
serialize_mods(STATE_MODS_EFFECTIVE)
```

because that collapses everything into one combined mask. Wayland wants the three separate components so clients can reconstruct the state properly.

---

# 11. Why did `KeyOutput` look so large?

My earlier `KeyOutput` carried:

```rust
device
keycode
xkb_keycode
keysym
composed
pressed
repeats
timestamp_usec
modifiers
modifiers_changed
```

That is maybe 48–64 bytes after padding. It is not heap allocated. It is returned by value on the stack and likely optimized into registers/fields.

At normal keyboard event rates this is not a meaningful performance cost.

However, it is too much conceptually. Some fields are already known by the caller, and some are derivable.

Leaner:

```rust
#[derive(Debug, Clone, Copy)]
pub struct TranslatedKey {
    pub keysym: xkb::Keysym,
    pub composed: Option<xkb::Keysym>,
    pub repeats: bool,
    pub modifiers: Option<Modifiers>,
}
```

Remove:

- `device`: the seat already knows `event.key`
- `xkb_keycode`: derivable as `evdev_keycode + 8`
- `timestamp_usec`: belongs to the input event / logical event
- `pressed`: belongs to the logical event
- `modifiers_changed`: encode it by `Option<Modifiers>`

Then the seat constructs the protocol-ready event:

```rust
LogicalEvent::Keyboard(KeyEvent {
    keycode: evdev_keycode,
    pressed,
    timestamp_usec,
    modifiers: self.modifiers.snapshot(),
    keysym: translated.keysym,
    composed: translated.composed,
})
```

Performance-wise, the bigger wins are:

- no Lua/config call on the event path
- no dynamic allocation per event
- reuse the `Vec<LogicalEvent>` with `clear()`
- avoid serializing modifiers on every key
- coalesce pointer motion

Not shaving 16 bytes off a stack struct.

---

# 12. Device-local modifiers vs seat-wide modifiers

Each `xkb::State` is device-local.

If you have two keyboards:

```text
Keyboard A: Shift held
Keyboard B: user presses A
```

The Wayland client has one focused keyboard for the seat. It should see modifier state including Shift from keyboard A.

So you need two layers:

```text
per-device xkb state  -> keysym resolution
seat-wide aggregation -> wl_keyboard.modifiers
```

The aggregator counts references per modifier bit:

```text
Shift on keyboard A down -> count[Shift] += 1
Shift on keyboard A up   -> count[Shift] -= 1
```

Seat snapshot:

```text
modifier bit is active if count[bit] > 0
```

This also handles hot-unplug correctly: when a keyboard disappears, remove all of its modifier contributions.

Layout/group is harder to merge. A simple policy is “last active keyboard wins”. That is acceptable initially.

---

# 13. Do you really need `DeviceInfo`?

For xkb processing: no.

For your Lua config idea: yes, in some form.

But it should not be duplicated into every subsystem. Have one owned identity record, preferably one registry:

```rust
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: Arc<str>,
    pub sysname: Arc<str>,
    pub seat: Arc<str>,
    pub capabilities: Capabilities,
}

struct DeviceRegistry {
    devices: SecondaryMap<DeviceKey, Arc<DeviceInfo>>,
}
```

Then:

```rust
enum LogicalEvent {
    DeviceAdded {
        key: DeviceKey,
        device: Arc<DeviceInfo>,
    },
    DeviceRemoved {
        key: DeviceKey,
        device: Arc<DeviceInfo>,
    },
    ...
}
```

Why `Arc<DeviceInfo>`?

Because `EventKind::DeviceAdded` currently borrows strings from libinput:

```rust
name: &'a str,
sysname: &'a str,
```

Those are valid only during the synchronous `dispatch` callback. If the config thread or Lua callback needs them later, you must own them.

`Arc<str>` gives you cheap sharing.

## Important config design point

Do not expose `DeviceKey` as stable device identity to Lua config files.

`DeviceKey` is a SlotMap key with a generation. It is perfect for correlating events inside one process lifetime, but not for persistent config.

Expose identity like:

```text
ctx.kb.name
ctx.kb.sysname
ctx.kb.seat
ctx.kb.capabilities
```

If you need `ctx.kb.port`, add a real `path`/`port` field to `DeviceInfo` later. `sysname` is often close, e.g. `event5`, but it is not guaranteed to be a stable USB port name.

Your Lua example should conceptually work like this:

```lua
Mux.bind(Mux.keyboard.added, function(ctx)
    print(ctx.kb.name, ctx.kb.sysname, ctx.kb.seat)

    if ctx.kb.name == "My Keyboard" then
        ctx.kb.layout("de")
    end
end)
```

Internally:

```text
Lua thread:
    register callback id

Compositor thread:
    DeviceAdded event
      -> create DeviceInfo
      -> register device
      -> emit DeviceAdded with Arc<DeviceInfo>
      -> notify config thread: “callback id X matched”

Lua callback:
    issues command SetKeymap { target, layout }

Compositor thread:
    applies SetKeymap by rebuilding keyboard state
```

No Lua call happens while processing input events.

That is exactly the right boundary.

---

# 14. What is `PolicySnapshot`?

`PolicySnapshot` is a frozen copy of config decisions that the input path is allowed to read.

Purpose:

```text
config thread / Lua may change settings
        |
        v
send new immutable snapshot to compositor thread
        |
        v
input processing reads local snapshot, no locks, no Lua
```

Example:

```rust
#[derive(Debug, Clone, Default)]
pub struct InputPolicy {
    pub keyboard: KeyboardPolicy,
    pub pointer: PointerPolicy,
    pub idle_timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct KeyboardPolicy {
    pub default_keymap: KeymapSpec,
    pub repeat: RepeatPolicy,
}

#[derive(Debug, Clone)]
pub struct PointerPolicy {
    pub speed: f64,
    pub accel: AccelerationMode,
    pub tap_to_click: bool,
    pub natural_scroll: bool,
}
```

Use it as:

```rust
policy: Arc<InputPolicy>
```

When config reloads, replace the `Arc`. Existing input processing sees either the old snapshot or the new one, never a half-updated mix.

On `DeviceAdded`, resolve policy once:

```text
DeviceInfo + global policy -> DevicePolicy for this DeviceKey
```

Store per-device overrides in a small `SecondaryMap<DeviceKey, DevicePolicy>`.

That is the clean split:

```text
DeviceInfo    = facts about the device
InputPolicy   = rules about what to do
DevicePolicy  = resolved rules for one live device
```

Bindings/callbacks should probably be separate from `InputPolicy`, because they are event handlers, not settings:

```rust
enum ConfigCommand {
    RegisterBind {
        trigger: Trigger,
        callback: CallbackId,
    },
    SetKeymap {
        target: DeviceTarget,
        spec: KeymapSpec,
    },
    SetPointerSpeed {
        target: DeviceTarget,
        speed: f64,
    },
}
```

---

# 15. Why does `Seat` have so many fields?

Because it is the place where device-local events become one coherent Wayland seat state.

Group them mentally:

```rust
struct Seat {
    // identity/lifetime
    devices: DeviceRegistry,

    // capability-specific state
    keyboards: KeyboardPool,
    pointers: PointerPool,

    // one state visible to Wayland clients
    modifiers: ModifierAggregator,

    // input batching / 1000 Hz motion
    motion: MotionCoalescer,
    pointer_events_this_batch: usize,

    // coarse timers
    idle: IdleTracker,

    // config snapshot
    policy: Arc<InputPolicy>,
}
```

Each field has a different purpose:

- `devices`: stable identity, config matching, logs
- `keyboards`: one xkb state per keyboard device
- `pointers`: per-device pointer settings/state
- `modifiers`: seat-wide aggregate sent to clients
- `motion`: coalesce many motion events into one `wl_pointer.motion`
- `pointer_events_this_batch`: know when to emit `wl_pointer.frame`
- `idle`: coarse activity timer, not a timer per event
- `policy`: config decisions without calling Lua on the hot path

You do not need all fields on day one. Build in this order:

1. `keyboards`
2. `pointers`
3. `modifiers`
4. `motion` / frame batching
5. `devices`
6. `policy`
7. `idle`

---

# 16. The abstraction I would aim for

Use these names:

```text
MOD_MASK                 -> WAYLAND_MODS_CHANGED
KeyOutput                -> TranslatedKey
Modifiers.layout         -> Modifiers.group
Keyboards                -> KeyboardPool
KeymapSpec               -> KeymapSpec / XkbRules
keymap()                 -> KeymapCache::get_or_compile()
PolicySnapshot           -> InputPolicy
DeviceInfo               -> DeviceIdentity or DeviceInfo
```

Minimal hot path:

```text
InputEvent
  -> Seat::process_input
      -> SecondaryMap::get_mut(device)
      -> xkb state key processing
      -> modifier aggregation
      -> push LogicalEvent
  -> Wayland serialization
```

Cold paths:

```text
DeviceAdded
  -> own DeviceInfo
  -> resolve policy
  -> compile/get keymap
  -> create keyboard state
  -> notify config thread

Config reload
  -> replace InputPolicy
  -> rebuild affected keyboard states

Lua bind
  -> register callback id
  -> event carries owned identity
  -> callback sends command back to compositor
```

That keeps the input path fast and leaves a clean place for your future Lua config without coupling xkb state to config callbacks.
