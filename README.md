# ospf

[![GitHub license](https://img.shields.io/badge/license-Apache%20License%202.0-green.svg?style=flat)](http://www.apache.org/licenses/LICENSE-2.0)
[![Maven Central](https://img.shields.io/maven-central/v/io.github.fuookami.ospf.kotlin/ospf-kotlin)](https://mvnrepository.com/artifact/io.github.fuookami.ospf.kotlin/ospf-kotlin)
[![Kotlin](https://img.shields.io/badge/Kotlin-1.9.22-yellow.svg?logo=kotlin)](http://kotlinlang.org)

## Introduction

ospf is a solution for the modeling and coding process in developing complex operational research algorithm software, along with its development components. It aims to provide a modeling approach based on <strong><em>Domain Driven Design</em></strong> (DDD), enabling users to efficiently develop and maintain mathematical models, solution algorithms, and their implementation code throughout the entire software lifecycle. For more detailed information, design or documentation, please refer to the documentation page:

ospf: https://github.com/fuookami/ospf

:us: English | :cn: [简体中文](README_ch.md)

## Intermediate Expression

ospf introduces a concept called "intermediate expression" to facilitate DDD-based modeling. Intermediate expressions are used in mathematical models to represent intermediate results of computations, aiming to simplify the representation of the model and make it easier to understand and maintain. Intermediate expressions possess the following characteristics:

- they refer to named polynomials that are stored.
- they are semantically equivalent to anonymous polynomials.
- they are grammatically equivalent to variables, having a global scope and static lifetime.
- they can be constructed through an anonymous polynomial.

<!-- add examples -->

## Components

ospf is designed and implemented using an internal <strong><em>Domain Specific Language</em></strong> (DSL) format. Apart from some shared components, the rest are implemented in the target host language.

### Shared Components

- <strong>[examples](examples)</strong>: examples demonstrating how to use OSPF for modeling and solving.

- <strong>[framework](framework)</strong>: a set of common components developed for specific problem domains, including visualization tools for results.

- <strong>[remote](remote)</strong>: a remote solver scheduler and server used to execute solvers on a server and retrieve results through a network interface.

### Components In Host Language Implementation 

each OSPF implementation consists of the following components:

- <strong>utils</strong>: utilities containing classes and functions required for implementing ospf DSL.
- <strong>core</strong>: core components containing essential functionalities such as modeling, solver interfaces, result processing, etc.
  - <strong>core-plugin-XXX</strong>: solver plugins implementing solver interfaces for specific solvers.
  - <strong>core-plugin-heuristic</strong>: meta-heuristic algorithm plugins containing implementations of various common meta-heuristic algorithms.
- <strong>framework</strong>: problem-specific frameworks containing implementations of data processing, mathematical models, and solving algorithms tailored to specific problems. All designs and implementations are non-intrusive, allowing users to use them out of the box or extend them seamlessly, integrating with other frameworks or components.
  - <strong>framework-plugin-XXX</strong>: framework plugins implementing functionalities requiring middleware involvement, such as data persistence, asynchronous message communication.
  - <strong>bpp1d</strong>: 1d bin packing problem development package containing implementations of data processing, mathematical models, and solving algorithms for various one-dimensional bin packing problems.
  - <strong>bpp2d</strong>: 2d bin packing problem development package containing implementations of data processing, mathematical models, and solving algorithms for various two-dimensional bin packing problems.
  - <strong>bpp3d</strong>: 3d bin packing problem development package containing implementations of data processing, mathematical models, and solving algorithms for various three-dimensional bin packing problems.
  - <strong>csp1d</strong>: 1d cutting stock problem development package containing implementations of data processing, mathematical models, and solving algorithms for various one-dimensional cutting stock problems.
  - <strong>csp2d</strong>: 2d cutting stock problem development package containing implementations of data processing, mathematical models, and solving algorithms for various two-dimensional cutting stock problems.
  - <strong>gantt-scheduling</strong>: gantt scheduling problem development package containing implementations of data processing, mathematical models, and solving algorithms for various Gantt chart scheduling problems. It can be used for scheduling and planning problems such as <strong><em>Advanced Production Scheduling</em></strong> (APS), <strong><em>Lot Scheduling Problem</em></strong> (LSP), etc.
  - <strong>network-scheduling</strong>: network scheduling problem development package containing implementations of data processing, mathematical models, and solving algorithms for various network scheduling problems. It can be used for scheduling and planning problems such as <strong><em>Vehicle Routing Problem</em></strong> (VRP), <strong><em>Facility Location Problem</em></strong> (FLP), etc.

## Features And Progress

### Core

### Framework

### Remote
