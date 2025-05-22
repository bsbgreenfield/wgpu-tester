# The current gltf processing pipeline

Writing this down because i need to find a cleaner way to organize the gltf processing pipeline. As it currently stands, it works like this:


- first we load the gltf file and create GModels. We find the "root nodes" in the gltf file, where each root node will indicate a model.
- using this root node, we traverse the json in the .gltf file to gather information on how many meshes each model contains, and how many 
instances of each mesh will be in each model.
- furthermore, each mesh has one or more associated primitive, which ultimately defines where the data is located through reference to 
an accessor, which in turn references a buffer view that has byte offset, length, and type data. 

## heres where the trouble starts
- When creating a mesh, we loop through the primitives associated with the mesh and store the byte offset and length in the GPrimitive struct
as defined by the buffer view for that primitive. Oftentimes, a primitive will reference slices of data that are also used elsewhere in the 
definition of the model.
- Our goal when creating the model is to have one continuous index buffer that the draw call can reference to when drawing the vertices.
if we do not handle duplicate index data, our indices will technically be correct, but the buffer will be way larger than necessary, and often
way larger than the GPU can even handle.
- To fix this, we need to add only unique data to the index buffer. The solution works as follows:

- while we are creating the primitives for the meshes, we are simultaneously building out a Vec<Range<usize>> to keep track of ranges that we
has seen before.

- for example, if a model has two primitives, and the data for primitive 1 is located in [100..136] and primitive 2's data is in [118, 154], 
then on instantiation of the first primitive we store [100..136] and on instantiation of the second primitive we store [100..154]. The range
is thus only expanded by 18 to accound for new *unique* data, so that the future index buffer doesnt have to track [118..136] twice. 

- primitives store the range in which their data is located on instantiation, but depending on how the main range vec has changed during 
the process of creating all other primitives, its data location relative to the final index buffer will be different. Luckily, its easy to 
determine where the data ended up. 
if the range vec ended up like this [0..10, 20..30], and a primitives data is located at [22..28], we know that because the index buffer
will tighly pack all its data, 22 will become 12, as the range [10..20] will not be included in the final buffer. 


1. we create all the primitives, and fill out scene_buffer_data with the correct index ranges
2. we call scene_buffer_data.init_indices. This grabs the appropriate slices of index data and packs them tightly
into one vec<u16>
3. The scene is instantiated, the index ranges relative to the main byte buffer is stored in scene.index_data
4. do the same for the second scene
5. merge scenes -> the two vec<u16>s are combined. The indices offset value for for all primitives in the second 
scene are incremented by the total length of the indices in the first scene
6. init scene -> the new index vec is created into a buffer
7. init models -> for each primitive in each model, we map the index offset of the primitive to the the correct location in the final buffer

example:
model 1: 
      index ranges: [0..3, 10..12] 
      buf: [1, 2, 3, 4, 5, 6, 7]
      primitives: {offset: 0, len: 4}, {offset: 10, len: 3}
model 2:
      index ranges: [2..5, 10..11] 
      buf: [1, 2, 3, 4, 5, 6]
      primitives: {offset: 2, len: 4}, {offset: 10, len: 2}
*model 2 primitive offsets are updated*
model 2 new primitives: {offset: 9, len: 4}, {offset: 17, len: 2}
merged index buf: [1, 2, 3, 4, 5, 6, 7, 1, 2, 3, 4, 5, 6]

*adjust the index offsets in init_models()*
model 1 primitives: {offset: 0, len: 4}, {offset: 5, len: 3}
model 2 primitives: 
    

we need to "initialize" primitives of each scene BEFORE they get to the merge code
Each primitives offset needs to be mapped to its new index within the composed buffer.
Then, when we merge the two scenes, all we have to do is increment the offset by the number
of indices in the previous scene.

so model 1s primiitves are {offset: 0, len: 4}, {offset: 4, len: 3}
model 2s primitives are {offset: 0, len: 4}, {offset: 4, len: 2}

and the merge code will turn those into {offset: 7, len: 4}, {offset: 11, len: 3}
which will actually corresponmd to the new buf
