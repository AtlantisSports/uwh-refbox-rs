# Domain Knowledge: UWH Refbox System

This document explains what the refbox system does in plain English. It is the primary reference
for understanding the *why* behind the code — what real-world problem each piece of software
solves. Future Claude sessions should read this before working on any feature or fix.

---

## What is Underwater Hockey?

> [USER INPUT NEEDED: A few sentences describing the sport — what it is, how it is played, and
> what makes it unique. This helps Claude understand why certain refbox features exist.]

---

## What is the Refbox?

The refbox (referee box) is software that manages an underwater hockey game in real time. During a
game, the referee operator uses it to:

- Track the game clock (start, stop, and pause play)
- Record goals scored by each team
- Record fouls and penalties against players
- Manage team timeouts
- Trigger buzzer sounds at key game events (start of play, end of half, timeouts)

The refbox is the authoritative source of game state during a match. Other components (the LED
panel, the stream overlay) receive their information from it.

---

## Hardware Setup

The refbox software runs on either a **laptop** or a **Raspberry Pi (RPi)**, depending on the
tournament. Both are fully supported.

### Components in a typical tournament setup

| Component | Software | Purpose |
|-----------|----------|---------|
| Refbox computer (laptop or RPi) | `refbox` | Main game management interface operated by the referee operator |
| LED panel (poolside display) | `led-panel` firmware | Physical scoreboard showing scores and time to spectators |
| Wireless remote (handheld button) | `wireless-remote` firmware | Lets the referee trigger buzzer sounds without touching the computer |
| Stream computer (major tournaments) | `overlay` | Displays a live game-state graphic on the broadcast video stream |

### How they connect

- The **wireless remote** communicates with the refbox computer over LoRa radio. When the referee
  presses a button, the refbox plays the appropriate buzzer sound.
- The **LED panel** receives display data from the refbox computer over a serial/USB connection.
- The **overlay** connects to the refbox over the local network and reads live game state to
  display on the stream.

---

## Game Flow

> [USER INPUT NEEDED: Walk through what the refbox operator does from arrival at the pool to the
> end of the game. What do they set up first? What do they click at each stage of the game?
> Describe the full sequence in plain language.]

Key stages to cover:
1. Pre-game setup (loading the schedule, selecting the game, entering teams)
2. Starting the first half
3. During play (recording goals, fouls, calling timeouts)
4. Half-time
5. Starting the second half
6. End of game (confirming the final score)
7. Between games (resetting for the next game)

---

## The UWH Portal

> [USER INPUT NEEDED: What is the UWH Portal? Is it a website run by the sport's governing body?
> Who enters data into it? What data does it hold?]

The `schedule-processor` is a command-line tool run by the tournament organizer before each
tournament. It:

1. Reads the tournament schedule (from the UWH Portal, exported as a CSV file or fetched via API)
2. Validates the schedule for common errors (team conflicts, missing data, etc.)
3. Generates scoresheets
4. Outputs the schedule in a format the refbox can load

---

## Key Game Concepts

> [USER INPUT NEEDED: Confirm or correct these definitions. Add anything that is missing.]

| Concept | Description |
|---------|-------------|
| **Half** | One period of play. A game has two halves. [USER INPUT NEEDED: typical duration?] |
| **Timeout** | A pause in play. [USER INPUT NEEDED: who can call one? how many per team? how long?] |
| **Penalty** | [USER INPUT NEEDED: what triggers a penalty? how is it served?] |
| **Foul** | [USER INPUT NEEDED: what is a foul? is it different from a penalty?] |
| **Coin toss** | Determines starting positions. The schedule-processor can resolve coin tosses. |
| **Game snapshot** | The internal data structure representing complete game state at a moment in time (scores, clock, player states, etc.) |
| **Placement / seeding** | How teams are ranked for finals. The schedule-processor handles "list of placements" logic. |

---

## Glossary

Technical terms used in the codebase that have specific meanings in this project:

| Term | Meaning |
|------|---------|
| `refbox` | The main referee software application |
| `snapshot` | A complete capture of game state at a point in time |
| `overlay` | The broadcast stream display application |
| `portal` | The UWH Portal online platform |
| `schedule-processor` | CLI tool for processing tournament schedules before a tournament |
| `wireless-remote` | Embedded firmware for the handheld referee button device |
| `led-panel` | RTL design for the physical LED scoreboard hardware |
| `matrix-drawing` | Code for rendering graphics on the LED panel display |
| `alphagen` | Utility for processing image assets used by the overlay |
| `uwh-common` | The shared Rust library containing core game types used by all other crates |
| `MSRV` | Minimum Supported Rust Version — the oldest Rust version this code must compile on (1.85) |
| `no_std` | Rust code that works without the standard library — required for embedded targets |
| `iced` | The GUI framework used to build the refbox user interface |
| `Embassy` | The async framework used in the wireless-remote embedded firmware |
| `RP2040` | The microcontroller chip inside the wireless remote (Raspberry Pi Pico) |
| `LoRa` | The radio protocol used for communication between the wireless remote and the refbox |
| `cross` | A tool for compiling the refbox for Raspberry Pi from a regular laptop or desktop |
