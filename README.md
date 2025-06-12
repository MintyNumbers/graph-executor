# graph-executor

Graph Executor Component implementation written in Rust.

## Description

This project implements a multi-processing graph executor component capable of executing directed acyclic graphs (DAGs), where each node represents an arbitrary computation (such as a function or task), and each edge denotes a dependency relationship &ndash; thereby ensuring nodes execute only after all prerequisite nodes have completed. The graph executor component is implemented in Rust, with a strong focus on inter-process communication via POSIX shared memory objects and robust synchronization using a semaphore-based MRSW lock.

Currently, nodes do not yet execute arbitrary supplied computations, but rather print a supplied "label".

## Getting Started

### Dependencies

You need to either have a Linux system or some variation of Docker or Podman installed in order to open the dev container, and build and use the graph executor component. Furthermore, VS Code is advisable to use along with the `Dev Containers` extension.

### Installing

To compile the graph executor component, the user has to follow these basic steps:

1. Clone the project's GitHub repository: `git clone git@github.com:MintyNumbers/graph-executor.git`.
2. Open VS Code and download the `Dev Containers` extension.
3. Open VS Code's command palette by either pressing `F1` or using the keyboard shortcut `Ctrl + Shift + P` and enter `Dev Containers: Build and Reopen in Container` in order to build the dev container and open the project inside of it.
4. After the dev container's build process#footnote[The initial build may take a few minutes; subsequent build processes are substantially faster because all the required resources are cached on the machine the dev container is running on.],the graph executor component can be compiled by simply entering `cargo build --release` into VS Code's terminal.


### Executing program

After compiling the graph executor binary, the user can execute the graphs in the project's top-level `resources` directory by executing the binary from a terminal, along with a specified path to a DOT file of the graph e.g. `./resources/example-typical-dot-digraph.dot` and `filename_suffix` for the POSIX shared memory objects. You can execute the compiled binary like this:
```bash
./target/release/graph-executor ./resources/example-typical-dot-digraph.dot filename_suffix
```

The output of the execution should be as follows, i.e. equal to the order specified in `./resources/example-typical-dot-digraph.dot`:
```
a
b
c
d
```

To define a custom graph, the user can use the widespread DOT syntax or the project's custom one. The custom syntax can be viewed at `./resources/example-printed-dot-digraph.dot` and allows for specifying a specific "label" which is printed to `stdout` during the graph's execution; if using the traditional DOT syntax, then the node identifier becomes the node's label and is printed instead (like in #ref(<code-graph-executor-component-execution>)).

Alternatively, the graph executor component's code can also be directly integrated into a Rust project by simply using the `graph_structure` module to programmatically define a DAG (or read it from a file) and then call its execution method in the Rust application. If needed, new threads or processes can be spawned to accelerate the execution.

Other than that, the previous chapters have also covered some minor usability issues of the current implementation, like the necessity to update the execution status of nodes in both: the `PosixSharedMemory` instance, as well as in the process context; or the fact that the processes of serializing and deserializing the `DirectedAcyclicGraph` instance take place at different points in the code.



## Acknowledgments

* [Simple Readme](https://gist.github.com/DomPizzie/7a5ff55ffa9081f2de27c315f5018afc)
* [Shared Memory and Semaphores with Rust](https://medium.com/@alfred.weirich/shared-memory-and-semaphores-with-rust-09435ca8c666)
* [Rust Atomics and Locks](https://marabos.nl/atomics/)
