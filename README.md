# tiny-rust-games
A repository for building very simple games using Rust and its various game engines.

The purpose of this repository is to hold example implementations of some very simple, well-known games. Each game is built with three goals in mind:

1. The code for the game should be transparent and idiomatic so that it can be used for educational purposes.
2. The code should be portable so that it can be used across many game libraries and distributed on many platforms. The code should be implemented in as many game libraries as possible as a proof of concept for each system.
3. The code should be extensible so that others can use it as a starter template for a more complicated game of their own.
4. The core game logic should be factored into its own library and game engine agnostic wherever this heuristic can be sensibly applied so that the same code can be used across multiple engines. Wherever this goal detracts from goal number one, goal number one should take precedence.
