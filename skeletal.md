## NOTES
- When applying transformations to nodes that are skinned (i.e. nodes that are a part of a hierarchy which contains a skinned mesh), 
we must not use the local transforms for these meshes at all. In other words, the pose of the vertices that compose a skinned mesh is entirely
determined by the transforms and weights of the associated joints.

- JOINTS_N is an attribute on a primitive which contains INDICES that correspond to one or more of the JOINTS defined on a SKIN

Ex. 
skins : {
     { 
         joints: [1, 2] 
     }
}

meshes: {
   {
     joints_0: 1 // accessor contains 1, 1, 1 for the three verices
   }, 
   {
     joints_0: 2 // accessor contains 2, 2, 2 for the three verices 
   }

}
