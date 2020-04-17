- Collect sources
- For each file:
    - Create module name
    - strip
    - parse
    - create module
    - insert into graph
    - find dependencies
    - insert edge for each dependency
- Sort topographically, error on cycle
- For each module:
    - Try to infer the type info for each module
    - Add type info for that module to map
    - Use the AST to build the `Out` struct
- For each compiled module:
    - remove type info from map and put into `Analyzed` struct.
- Return list of `Analyzed` structs
- For each `Analyzed`:
    - generate erlang files
- For each erlang file:
    - write to disk


# Compile project

- clear build directory
- collect sources
- input into db
- project_erlang_files
    - get_all_compiled
        - get_all_source_files
            - get_all_sources
            - for each source:
                - get_source_file
        - for each source:
            - get_ast
                - strip
                - parse
            - create module
        - return modules
    - check for import cycle
    - for each module:
        - get_module_type_info
            - get deps
            - for each dep:
                - get_module_type_info
            - infer_module
        - create `Analyzed`
        - generate_erlang
        - return `Vec<OutputFile>`
- for each file:
    - write to disk


# Incremental Compile

- Compile project
- listen for file changes
- on each change:
    - project_erlang_files
    - for each file:
        - if updated, write

# LS ops


        
