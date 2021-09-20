# GroundhogGame

## Introduction

A Rust/Bevy ECS game that uses autonomous agents to build out a small, deterministic open world.

This is primarily divided into an [engine](engine) which manages game processes, and [source scripts](src) which define the initial world state. The engine has been further divided into a few pieces, but that is still a work in progress. Several files are duplicated as a result of that work. 

The python/ directory contains an earlier text-based attempt which runs too slowly to be useful and has some architectural issues. It is included for reference purposes for further development, as much of the logic has not been implemented in the rust version. 

In its present state, this is a stress test of the pathing system.

Immediate goals are optimization of the current system and reorganization of modules into a cleaner hierarchy. 