# GroundhogGame

## Introduction

A Rust/Bevy ECS game that uses autonomous agents to build out a small, deterministic open world.

The python/ directory contains an earlier text-based attempt which runs too slowly to be useful and has some architectural issues. It is included for reference purposes for further development, as much of the logic has not been implemented in the rust version.

## Structure

At the moment, the engine is divided into several subcategories.

### main.rs

This is the script which loads the game. Presently, it initializes all of the world resources and spawns actors. In the near future, most world building will be moved into a serialized format and parsed here instead.

### actor

The [actor module](engine/actor) is responsible for handling actor state and game ai. Actor state includes statuses (Hunger, Thirst, Health) and [not implemented] individual stats (Str, Int, etc.). Game AI works as follows:
An actor's goal is determined by their needs via statuses and [not implemented] routines, which direct action over the course of the day. This goal is added to the actor as a component.
Each goal has a companion system that controls flow through the tasks needed to achieve that goal. The current task is added to the actor as a component by this system. Subsequent task systems are responsible for removing the task when it is concluded.

When an actor's destination changes, the pathfinding system finds a new path for them using A\*. When entities are in close proximity, the local avoidance system navigates them around the others.

### world

The [world module](engine/world) is responsible for maintaining the world state. It holds a map of tile weights for pathfinding and a map of occupied tiles, and provides information on which tiles are accessible from a given position. It manages time through GameTime objects which track internal time state. This allows the game to flow at a prescribed rate and [not implemented] handles catch-up when systems start to run behind. It holds data relating to game objects.

### render

The render module updates sprites based on actor orientation. Because I have not yet settled on a graphical style, this is all it will do for the near future.

### ui

The ui module does nothing. The code inside was not written by me, and was used for testing. BevyEngine's UI implementation will probably be subject to big changes, and isn't especially sophisticated at the moment. Resultantly, UI will be implemented at a later stage.

## Determinism

Performant determinism in an otherwise parallel program is a tricky problem. Often, two entities will attempt to access one resource (right now, that resource is tiles). My current solution is to prepare a list of the current frame's changes to the world state, identify and address conflicts when they arise, and then process them. Conflicts will eventually be addressed based on properties of the entities. This is not yet implemented because all of my test entities are identical. Instead, resources are handed out on a first-come first-serve basis, and the result is non-performant non-determinism.

Alternatively, resource conflicts may be resolved as a change to game state; two actors moving to the same tile could bump into each other, remain at their previous position, and become dazed.

Undoubtedly, there are standing issues with determinism. Until the program is more stable, I am not writing tests for determinism because that work will be undone.
