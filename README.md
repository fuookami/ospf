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

### Arithmetic Intermediate Expression:

The initial purpose of designing intermediate expressions was to reduce redundancy in mathematical models. Therefore, the most basic arithmetic intermediate expressions are constructed using a polynomial, allowing users to replace all identical polynomials with this intermediate expression anywhere in the model.

$$
ExprSymbol = \sum_{i} x_{i} \\\\\\ \\\; \\\\\\
min \quad ExprSymbol \\\\\\ \\\; \\\\\\
s.t. \quad ExprSymbol \leq 1
$$

ospf automatically replaces each arithmetic intermediate expressions with specific polynomials when translating the model into interfaces for specific solvers. This translation process is transparent to the user, so the user does not need to know how the arithmetic intermediate expressions are implemented through which variables and operations.

Thus, we can divide the maintainers of mathematical models into two roles: <em>"intermediate expressions maintainer"</em> and <em>"user of intermediate expressions"</em>. The intermediate expressions maintainers are responsible for defining and implementing intermediate expressions, while users of intermediate expressions do not concern themselves with the implementation of intermediate expresssions. They only focus on the definition and behavior of intermediate expressions and use these intermediate expressions to describe business logic in mathematical models.

This engineering practice is similar to <strong><em>Object-Oriented Design</em></strong> (OOD), where defining a class encapsulates variables and functions with the same semantics, and users only need to focus on their behavior without concerning themselves with their implementation. With such a foundation, we can then begin to introduce DDD.

### Functional Intermediate Expression:

Building upon the concept of arithmetic intermediate expressions, ospf can also encapsulate non-arithmetic expressions such as logical operations into intermediate expressions.

$$
FuncSymbol = \bigvee_{i} x_{i} = Or(x_{1}, \, x_{2}, \, .. \, , \, x_{i}) \\\\\\ \\\; \\\\\\
s.t. \quad FuncSymbol = 1
$$

When ospf translates the model into interfaces for specific solvers, it automatically adds the intermediate variables and constraints required for each intermediate expressions. This translation process is transparent to the users, so the users don't need to know how the intermediate expressions are implemented through which intermediate variables and constraints. For example, the expression $FuncSymbol = \bigvee_{i} x_{i}$ will be translated as follows:

$$
s.t. \quad y = 1 \\\\\\ \\\; \\\\\\
\begin{cases}
  y \geq \frac{x_{i}}{\sup_{\leq}(x_{i})}, & \sup_{\leq}(x_{i}) > 1 \\\\\\
  y \geq x_{i}, & else
\end{cases} \\\\\\ \\\; \\\\\\
y \leq \sum_{i} x_{i}, \; \forall i \\\\\\ \\\; \\\\\\
y \in \{ 0, 1 \}
$$

Of course, you can also extend these intermediate expressions according to your own business requirements. At this point, you need to implement some interfaces to let ospf know which intermediate variables and constraints need to be added for these intermediate expressions.

The <em>ospf-core</em> only maintains arithmetic and logical functions. In fact, we can design and implement functional intermediate expressions based entirely on the domain, as part of domain engineering. For specific references, you can refer to the development package for specific problem domains in the <em>ospf-framework</em>.

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
  - <strong>bpp1d</strong>: 1D <strong><em>Bin Packing Problem</em></strong> (BPP) development package containing implementations of data processing, mathematical models, and solving algorithms for various 1D BPPs.
  - <strong>bpp2d</strong>: 2D <strong><em>Bin Packing Problem</em></strong> (BPP) development package containing implementations of data processing, mathematical models, and solving algorithms for various 2D BPPs.
  - <strong>bpp3d</strong>: 3D <strong><em>Bin Packing Problem</em></strong> (BPP) development package containing implementations of data processing, mathematical models, and solving algorithms for various 3D BPPs.
  - <strong>csp1d</strong>: 1D <strong><em>Cutting Stock Problem</em></strong> (CSP) development package containing implementations of data processing, mathematical models, and solving algorithms for various 1D CSPs.
  - <strong>csp2d</strong>: 2D <strong><em>Cutting Stock Problem</em></strong> (CSP) development package containing implementations of data processing, mathematical models, and solving algorithms for various 2D CSPs.
  - <strong>gantt-scheduling</strong>: gantt scheduling problem development package containing implementations of data processing, mathematical models, and solving algorithms for various Gantt chart scheduling problems. It can be used for scheduling and planning problems such as <strong><em>Advanced Production Scheduling</em></strong> (APS), <strong><em>Lot Scheduling Problem</em></strong> (LSP), etc.
  - <strong>network-scheduling</strong>: network scheduling problem development package containing implementations of data processing, mathematical models, and solving algorithms for various network scheduling problems. It can be used for scheduling and planning problems such as <strong><em>Vehicle Routing Problem</em></strong> (VRP), <strong><em>Facility Location Problem</em></strong> (FLP), etc.

## Features And Progress

### Core

### Framework

### Remote
