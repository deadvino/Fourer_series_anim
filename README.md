# Fourier Series Visualizer

A cross-platform desktop app for visualizing how Fourier series converge to a  
function, term by term, with real-time animation.
Built in Rust with [eframe/egui](https://github.com/emilk/egui) for the UI and  
[meval](https://github.com/ebrian/meval-rs) for parsing math expressions.

Another vibe-coded project...

## What it does

You provide the Fourier coefficients directly as expressions:

```
f(x) = a₀/2 + Σ [ aₙ·cos(nx) + bₙ·sin(nx) ]
```

*   Enter `a₀` as a constant expression (e.g. `2/pi`)
    
*   Enter `aₙ` and `bₙ` as expressions in `n` (e.g. `2*sin(n)/(pi*n)`)
    
*   Press **Start** and the app animates the partial sums — one harmonic added  
    at a time — so you can watch the series converge to the target waveform
    

## Features

*   **Live expression parsing** — coefficients are compiled once with `meval`  
    and re-evaluated only when changed
    
*   **Animated partial sums** — 0.1 to 250 terms/second, with start/pause/reset
    
*   **Incremental evaluation** — each frame adds exactly one term to the  
    existing curve instead of recomputing the whole sum (O(points) per term)
    
*   **Interactive plot** — drag, zoom, and scroll via `egui_plot`
    
*   **Configurable domain** — adjustable x-range and plot resolution  
    (10–3000 points)
    
*   **Up to 10,000 terms** per series
    
*   **Inline error reporting** — invalid expressions are shown in the sidebar  
    without crashing
    

## Screenshots / examples

![Example_step_function_screenshot](example_screenshot.png?raw=true "Example step function")

Default preset (sinc-like coefficients):

```
a₀ = 2/pi
aₙ = 2*sin(n)/(pi*n)
bₙ = 0
```

Other examples you can paste in:

```
4*(-1)^n/n^2
2*pi^2/3
```

## Getting the executable (no Rust required)

Download the latest release for your platform from the  
[Releases page](../../releases):

*   Windows: `fourier-visualizer.exe`

Just run it — no installation needed.

## Building from source

### Prerequisites

*   [Rust](https://rustup.rs) (stable, 1.75+)
    
*   Linux only: `sudo apt install libgl1-mesa-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-dev libwayland-dev libgtk-3-dev`


## Dependencies

*   `eframe` — immediate-mode GUI framework (egui backend)
    
*   `egui_plot` — plotting widget for egui
    
*   `meval` — math expression parser/evaluator (compiles coefficient  
    expressions into fast closures via `Expr::bind("n")`)
    

## Architecture notes

*   `compile_coefficients()` parses the three input strings once and stores  
    boxed closures from `meval`, so per-term evaluation never re-parses.
    
*   `add_term(n)` evaluates `aₙ`/`bₙ` once per term and accumulates into the  
    shared `y_values` buffer — adding terms is cheap, so high animation speeds  
    and large term counts stay smooth.
    
*   Repainting is only requested while animating, so the app idles at ~0% CPU.