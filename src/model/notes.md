to dynamically find root nodes instead of using scene.nodes
```rust
    fn get_root_nodes(g_nodes: &Vec<NodeWrapper>) -> Vec<usize> {
        let mut children = HashSet::<usize>::new();
        for node in g_nodes {
            for i in node.child_indices.iter() {
                children.insert(*i);
            }
        }
        // root nodes are the indices from 0 to g_nodes.len() that aren't in this list
        let mut root_nodes_ids = Vec::<usize>::new();
        for i in 0..g_nodes.len() {
            if !children.contains(&i) {
                root_nodes_ids.push(i);
            }
        }
        root_nodes_ids
    }
```


The end goal is to have one vertex buffer containing all of the data contained in all the object's meshes

if there are two meshes, like in the cesium milk truck, the vertex buffer should be 

[ mesh 0 data | mesh 1 data ]

and the same for the index buffer

when calling RenderPass.draw_indexed() for a given mesh, i need to pass in 
- num_elements = current_mesh.vertices_offset .. current_mesh.vertices_len + current_mesh.vertices_offset
- base_vertex = current_mesh.vertices_offset
- instances = current_model.mesh_instances[mesh index]

this implies that i need a GModel struct that looks something like this

```rust
struct Scene {
    models: Vec<GModel>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer, 
    instance_data: InstanceData, // custom struct that holds all needed instance data
    camera: Camera, // custom struct holding al needed datra for camera
}

struct GModel {
    data_buffer: Rc<Vec<u8>>,
    meshes: Vec<GMesh>,
    mesh_instances: Vec<usize> // length must equal the length of [meshes]
}

struct GMesh {
    vertex_offset: u32, // the offet of the mesh within the vertex buffer
    indices_offset: u32,
    indices_length: u32 // the number of indices in the mesh
}
```
then I can implement draw_mesh_instanced and draw_model_instanced for RenderPass

```rust
fn draw_mesh_instanced(&mut self, mesh: GMesh, instances: usize) {
    let indices = mesh.indices_offset.. (mesh.indices_offset + mesh.indices_length);
    self.draw_indexed(indices, mesh.vertex_offset, 0..instances);
}

fn draw_model_instanced(&mut self, model: GModel) {
    for (idx, mesh) in model.meshes.iter().enumerate() {
        let mesh_instances = model.mesh_instances[idx].clone();
        self.draw_mesh_instanced(mesh, mesh_instances);
    }
}
```
## important caveat to drawing meshes

The above reasoning for drawing models is correct. However, I overlooked an important 
aspect of drawing individial meshes. Namely, that a mesh may contain multiple primitives. 

Futhermore, these primitives may required different buffer layouts, lighting techniques, etc,
which would require different *shaders*. For now, i will be ignoring this additional complexity 
and assuming that each primitive in the mesh will adhere to the same buffer layout and receive 
same post processing steps, so that i only need a single pipeline to process everything.

Even still, I need to break the draw_mesh function down into the individiual steps needed to
process each set of data associated with each primative.

The plan:
- I will still compose one large vertex buffer and one large index buffer for each model.
- in the draw_mesh_instanced function, i will loop through the primitives of the mesh 
and call render_pass.set_vertex_buffer() and render_pass.set_index_buffer() with a slice of the 
larger buffers that corresponds to where the data actually lies for that primitive. 
- It follows that i need a new struct "GPrimitive", which will look like this:

```rust
struct GPrimitive {
    vertex_offset: u32, 
    vertex_len: u32,
    index_offset: u32,
    index_len: u32
    // normals?
    //material?
}
```

and so draw_meshed_instanced will be something like
```rust

fn draw_mesh_instanced(&mut self, mesh: GMesh, instances: usize, vertex_buffer: &wgpu::Buffer, index_buffer: &wgpu::Buffer,) {
  for primitive in mesh.primitives {
        self.set_vertex_buffer(1, vertex_buffer.slice(primative.vertex_offset.. primative.vertex_offset + primitive.vertex_len) );
        self.set_index_buffer(1, vertex_buffer.slice(primative.index_offset.. primative.index_offset + primitive.index_len) );
        self.draw_indexed(vertex_buffer.slice(primative.index_offset.. primative.index_offset + primitive.index_len, 0, instances );
    } 
}
```

## local and global transformations

When drawing meshes, we need to apply a transform to position each mesh correctly with respect to its local coordinate system as
indicated by the transform matrix defined in the corresponding node. 

Therefore, we need a local transform buffer that has one value per each mesh that we are drawing. 

However, we arent "drawing" meshes, we are drawing primitives. 
In the cesium milk truck exmpample:

* draw mesh 0 
    * draw primitive 0
        * multiply each vertex by the transform at index 0 in local transform buffer
* draw mesh 0 again
    * draw primitive 0
        * multiply each vertex by transform index 1

* draw mesh 1
    * draw primitive 0
       * multiply each vertex by transform index 2
    * draw pimitive 1
        * multiply each vertex by transorm index 2
    * draw primitive 2
       * multiply each vertex by transform index 2

so we need a way to communicate to the gpu that each instance of a primitive needs to correspond to the transform for the
mesh of which it is a part.

tldr: local transform buffer contains one value per mesh instance, each primitive in that mesh instance get multiplied by that value. 


# local transform bug
- for the cesium milk truck we are looping through two meshes and calling self.draw_gmesh_instaced on each
- each mesh is drawn with a separate draw call to each of its primitives.

the bug was that i wasnt passing in the correct range to the transforms. The instances range needs to correspond to the
indices within the local transform buffer, not just 0 to the number of instances.
