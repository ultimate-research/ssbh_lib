# SSBH Offset Rules - WIP
* relative ofsets in array elements point past the array
* offset fields in a struct point past the end of the struct
* if relative offset field 1 appears before relative offset field 2 in the struct, relative offset 1 points to a smaller address than relative offset 2 (order is preserved)
* array relative offsets are 8 byte aligned
* string data is 4 or 8 byte aligned
* other types may have their own alignment rules

### SSBH Exporter Pseudocode
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