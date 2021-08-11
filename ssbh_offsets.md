# SSBH Offsets
The SSBH formats exclusively use relative offsets that point relative to the start of the pointer type. All offset values are assumed to be in bytes. For example, if a 64 bit offset is defined starting at position 8 and contains an offset value of 16, the data pointed to by the offset will be stored at location 8 + 16 = 24. This means that relative offsets should always be at least 8. The special offset value of 0 is reserved to represent no data or null.

Data should never overlap, which produces the following rules for relative offsets.
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

### SSBH Exporter Pseudocode
TODO: Rework this section to provide pseudocode for SSBHWrite.

The parsing template can be implemented using runtime reflection in languages that support it. Generating the code at design/build time using macros or templates will result in more readable code with less performance overhead at the cost of being more verbose.

Anything marked as `#code` represents source code that should be generated or written manually. Lines not marked `#code` can be computed at design time using a template or macro functionality.

Empty or NULL arrays have an offset and size of 0. Empty strings are represented as 4 bytes of 0 or 0x00000000 due to string data typically being 4 byte aligned. Some strings are 8 byte aligned.

```javascript
// The absolute offset of the next data location.
data_ptr = sizeof(ssbh_data) #code
// Start writing the fields of ssbh_data
write_struct(ssbh_data, data_ptr)

// Write or generate code for the struct fields recursively.
write_struct(struct_data, data_ptr)
    if struct_data is a primitive type
        write(struct_data) #code
    else if struct_data is array
        // Align the data_ptr and calculate the relative offset.
        data_ptr = align_to_eight(data_ptr, field) #code
        if struct_data is empty: #code
            write(0) #code
            write(0) # code
        else # code
            write(data_ptr - current_position()) #code
            write(length(struct_data.elements)) #code

        // Start writing the array data.
        current = current_position() #code
        seek(data_ptr) #code

        // data_ptr should point past the end of the array.
        data_ptr += sizeof(element_type(struct_data)) * length(struct_data.elements) #code

        // Assume the code to write the element type is already generated using write_struct.
        for element in struct_data.elements: #code
            write_element(element, data_ptr) #code 

        // Continue writing the rest of the fields.
        seek(current) #code
    else:
        for field in struct_data
            if field is primitive type
                write(field) #code
            if field is relative offset or field is string
                // Align the data_ptr and calculate the relative offset.
                // Strings and other types are typically 4 or 8 byte aligned. 
                data_ptr = align_to_n(data_ptr, field, field_alignment) #code
                write(data_ptr - current_position()) #code

                // Write the data and update the data_ptr.
                current = current_position() #code
                seek(data_ptr) #code
                write_struct(field, data_ptr) #code
                data_ptr = current_position() #code
                seek(current) #code
            else
                // Recurse into the fields of field.
                write_struct(field, data_ptr) 
```

