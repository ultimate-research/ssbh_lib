# SSBH Offset Rules - WIP
* offsets in array elements point past the array
* offset fields in a struct point past the struct
* if offset 1 appears before offset 2 in the struct, the data for offset 1 will appear before the data of offset 2
* data is 4 byte aligned

### SSBH Exporter Pseudo Code
The parsing template can be implemented using runtime reflection in languages that support it. Generating the code at design/build time using macros or templates will result in more readable code with less performance overhead at the cost of being more verbose.

Anything marked as `#code` represents source code that should be generated or written manually. Lines not marked `#code` can be computed at design time using a template or macro functionality.

```python
# The absolute offset of the next data location.
data_ptr = sizeof(ssbh_data) #code
write_struct(ssbh_data)

def is_offset(data):
    return data is SsbhString or data is RelPtr64

def pad_and_align(absolute_offset, data):
    # TODO: establish alignment rules for strings and other types
    return align_to_four(absolute_offset)

# Write or generate code for the struct fields recursively.
def write_struct(struct_data):
    if is_primitive_type(field):
        write(field) #code
    else if is_array(struct_data):
        # Align the data_ptr and calculate the relative offset.
        data_ptr = pad_and_align(data_ptr, field) #code
        write(data_ptr - current_position()) #code
        write(len(struct_data.elements)) #code

        # Write the array data.
        current = current_position() #code
        seek(data_ptr) #code

        # data_ptr should point past the end of the array.
        data_ptr += sizeof(element_type(struct_data)) * len(struct_data.elements) #code
        for element in struct_data.elements: #code
            write_struct(element)
        seek(current) #code
    else:
        for field in struct_data:
            if is_primitive_type(field):
                write(field) #code
            if is_offset(field):
                # Align the data_ptr and calculate the relative offset.
                data_ptr = pad_and_align(data_ptr, field) #code
                write(data_ptr - current_position()) #code

                # Write the data and update the data_ptr.
                current = current_position() #code
                seek(data_ptr) #code
                write_struct(field) #code
                data_ptr = current_position() #code
                seek(current) #code
            else:
                write_struct(field)
```