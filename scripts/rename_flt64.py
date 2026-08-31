#!/usr/bin/env python3
"""
Rename non-generic VariableData/VariableItem/VariableArena to Flt64-prefixed versions
in variable_arena.rs, while preserving generic versions (VariableData<VT>, etc.)
"""
import re

filepath = r"E:\workspace\ospf-rust\ospf-rust-core\src\variable\variable_arena.rs"

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace non-generic references:
# VariableData (not followed by <) -> Flt64VariableData
# VariableItem (not followed by <) -> Flt64VariableItem
# VariableArena (not followed by <) -> Flt64VariableArena

def rename_non_generic(content, name, prefix="Flt64"):
    """Replace `name` with `prefix+name` only when NOT followed by `<`"""
    pattern = r'\b' + re.escape(name) + r'(?![<\w])'
    return re.sub(pattern, prefix + name, content)

content = rename_non_generic(content, "VariableData")
content = rename_non_generic(content, "VariableItem")
content = rename_non_generic(content, "VariableArena")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Done")
