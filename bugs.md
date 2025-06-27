 I think i may have found the error that is causing the meshes to render correctly, but in the wrong location.
What seems to be happening, is that i am not ordering the local transforms in the way that the draw passes expect them to be ordered.


when drawing, the logic is this using the first three meshes of buggy as an example. 

1. loop through model.meshes, which is a vec of unique GMeshes with ids:
                     [2, 1, 0]

2. for each of these meshes, grab the number of times that this mesh ought to be drawn.
For this model, all three meshes have 2 instances.

3. call draw_gmesh_instanced with instances = 0..2 because this is the first mesh on the first model

4. each model has one primitive, so draw_gmesh instance tries to draw two instances of each meshes primitive
that means that we do 
draw(offset 0)
increment local tansform buffer by 1
draw(offset 1)

BUT the local transform buffer isnt laid out this way. It SHOULD be 

[(mesh 1 instance 1), (mesh 1, instance 2), (mesh 2 instance 1), (mesh 2 instance 2) ...]

but instead it is

[(mesh 1 instance 1), (mesh 2, instance 1), (mesh 3 instance 1), (mesh 1 instance 2) ...]

the challenge is that we want the local transforms to be laid out with all the like meshes contiguous, but 
we need to account for any visiting order. And we CANT adjust the visiting order, but that is of course instrinsic
to the values of the local transforms themselves. 

So the solution I think is to keep the find model meshes function the way it is, because we are still calculating the 
correct values, but instead we need to not just greedily push the mesh transform straight into the lt buffer.

So, the order of the unique meshes can still be determined by the visiting order of the node tree,
but if a node has already been visited, we still make sure it is right next to the fist instance in memory

this can be acomplished with a series of mesh "buckets"
```rust
type MeshBuckets  = Vec<Vec<[[f32;4];4]>>;
```
