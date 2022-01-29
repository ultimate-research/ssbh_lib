# SSBH Offsets
The SSBH formats exclusively use relative offsets that point relative to the start of the pointer type. All offset values are assumed to be in bytes. For example, if a 64 bit offset is defined starting at position 8 and contains an offset value of 16, the data pointed to by the offset will be stored at location 8 + 16 = 24. This means that relative offsets should always be at least 8. The special offset value of 0 is reserved to represent no data or null.

Data should never overlap, which produces the following rules for relative offsets.  

0. An offset of 0 encodes null or no value. Empty or null arrays have an offset and size of 0. Null strings are simply a relative offset of 0. Empty strings are represented as N bytes of 0 where N is the alignment of the string data.
1. Offsets point past the containing type. For arrays, the offset will point past the end of the array. For structs, the offset will point past the struct. This disallows any kind of shared references or self referential structs. In addition, all offset values are non negative.
2. If offset1 appears before offset2, the data pointed to by offset1 will appear before the data pointed to by offset2. This means that data is stored in the same order that the offsets are stored.
3. The computed absolute position (offset position + offset value) will be the smallest offset value that obeys the minimum alignment of the pointed to type. The alignment for most types is 8 bytes.

Rule 3. is important in that it allows for computing the size of a type after any padding or alignment is applied. Consider the following attempt to determine a new type in an SSBH format.
```rust
struct MyData {
    unk1: u32,
    unk_offset: u64,
    // additional fields?
}
```
The value in `unk_offset` is the smallest value that points past `MyData` while obeying the alignment rules of whatever is pointed to by `unk_offset`. Suppose the `MyData` struct starts at position 16 in the file and the value of `unk_offset` is 64. `unk_offset` is at position `16 + 4`, so it points to file position `16 + 4 + 64 = 84`. This gives an upper bound of `84 - 16 = 68` bytes for the size of `MyData`. The actual size of `MyData` may be slightly smaller since this estimate takes into account the minimum alignment of the type pointed to by `unk_offset`.

A similar process can be applied to elements in an `SsbhArray`. If an array element contains an offset, it will point past the array by Rule 2, so this gives an upper bound on the size of the array's elements. Dividing the number of bytes for the array by the number of elements gives an estimate on the bytes per element.

Taking into account all three rules gives a simple implementation that can be reused across SSBH formats and handles cases that would be difficult to write out by hand such as nested offsets.

### SSBH Writer Pseudocode
A simplified version of the implementation used for the SsbhWrite crate is presented below as Python methods. Some details like the implementation of the writer object are omitted. The logic is extremely repetitive, so use code generation or reflection when possible to avoid any errors from hand typing the code. 

```python
# The write method for all types with multiple properties or fields.
# Omitting the data_ptr check at the beginning may mess up offset calculations.
def ssbh_write(self, writer, data_ptr)
    # No overlapping data (rule 1).
    current_pos = writer.position()
    if data_ptr < current_pos + self.size_in_bytes():
        data_ptr = current_pos + self.size_in_bytes()

    # Write all the fields in order.
    # This covers rule 2 since all types use the position check above.
    self.field1.ssbh_write(writer, data_ptr)
    self.field2.ssbh_write(writer, data_ptr)
    # ... continue for remaining fields. 
```

```python
# Recursively compute the size of the type.
# This is the size of the type's binary representation in an SSBH file.
# The size of the struct, class, etc may be different.
def size_in_bytes(self):
    size = 0
    size += self.size_in_bytes()
    size += self.size_in_bytes()
    # ... continue for remaining fields
    # Account for any padding or alignment.
    return size
```

```python
# The minimum alignment of the data_ptr when writing a pointer to this type.
def alignment_in_bytes(self):
    # This depends on the alignment of the various fields.
    # Strings can be 4 or 8 byte aligned.
    return 8
```

```python
# An example implementation of ssbh_write for a pointer type.
# SSBH arrays use similar logic but also need to write the array length.
def ssbh_write(self, writer, data_ptr):
    # No overlapping data (rule 1).
    # self in this case is the pointer or offset itself.
    current_pos = writer.position()
    if data_ptr < current_pos + self.size_in_bytes():
        data_ptr = current_pos + self.size_in_bytes()

    # Handle null offsets if the pointed to value is null (rule 0).
    if self.value is None:
        write_integer(writer, 0)
    else:
        # Calculate the relative offset by applying the pointed to type's alignment.
        data_ptr = round_up(data_ptr, self.value.alignment_in_bytes())
        relative_offset = data_ptr - current_pos
        write_integer(writer, relative_offset)

        # Save the position after the offset or after the length for arrays.
        saved_pos = writer.position()

        # Write the data at the specified offset.
        writer.seek(data_ptr)

        self.value.ssbh_write(writer, data_ptr)

        # Update the data pointer just in case self.value did not.
        # This is important when using optimized implementations for primitives, byte arrays, etc.
        let current_pos = writer.position()
        if current_pos > data_ptr:
            data_ptr = round_up(current_pos, alignment)

        # Move the cursor back to continue writing.
        writer.seek(saved_pos)
```