 so probably the way this should work is 

AppState::update() -> getAnimationFrame(currentTime) // async?

where 
```rust
struct AnimationFrame {
    lt_indices: Vec<usize>,
    transforms: Vec<LocalTransform>
}
```

then
for each animation in scene.allAnimations {
    animation.getFrameData(currentTime)
}
where 


```rust

struct Animation {
    times: Vec<f32>,
    translations: Vec<vec4>, //?
    lt_index: usize,
}

impl Animation {
    fn getFrameData(&self) -> (usize, LocalTransform) {
        let (prev, next) = self.getTranslations(currentTime);
        (self.lt_index, Self::calculateTranslation(self.interpolation, prev, next))
    }

    fn getTranslations(&self, currentTime: f32) -> (vec4, vec4) {
        let (index1, index2) = self.times.indexOf(currentTime);
        (self.translations[index1], self.translations[index2])
    }

    fn calulateTranslation(interpolation: Interpolation, prev: vec4, next: vec4) -> LocalTransform {
        match interpolation {
            Interpolation::Linear => linerTransform,
            Interpolation::cubic => cubicTransform, 
            _ => ...
        }
    }
}
```
alternatively, we could pass in a closure to the Animation itself to run? probably not necessary...

when creating the scene, all we should really have to do is make sure to assign the correct indices to the
animations which correspond to the mesh instances index within the local transform buffer

optimizations are: 
1. making the getAnimationFrame function async, so we can query all the animations in parallel
2. make sure that its really fast to update the local transform data vec with the new transforms for the current frame
3. make sure that its really fast to create and assign the new lt buffer

theoretically both 1 and 2 could beasync, and creating the new lt buffer can happen only once 
every value in the lt data vec has been successsfully updated, for which updates would happen as soon as an 
Animation has finished calculating its interpolated value and doing the matrix multiplication at the 
correct index.

Feels like this should be possible, but to avoid data races we would need to make sure that each animation has 
a unique lt index. I can imagine a situation where animations work on top of other animations, so technically 
we would have to do more than one transform to a single mesh...

not sure how common this is, but it could warrant some kind of composite version of the Interpolation enum, where
an interpolation could be like (Liner | Linear) or (cubic | linear) so that we need to vec4s to calculate the final
transform. But i would imagine that we could pull that off.

also, if we end up in a scenario where animations affecting other animations is super common, then we could maye still do 
async by splitting the animation caculations into multipl passes. or just doing it synchronously is always something to try



there is a somewhat tricky aspect to making sure the correct animations are applied to the correct meshes that I need to account for. Namely, the fact that animations are applied per node and not per mesh.

This means that there could, for example, be some node N

N {
  rotation: [...]
  children: [m1, m2]
}
with meshes 
m1 {
  translation: [..],
  mesh: 0
}
m2 {
  tranlation: [..],
  mesh: 1,
}

Now, if the animation is being applied to N, then it is saying that the rotation component of the local transforms
of both m1 and m2 are being update each key frame. This makes a lot of sense for a transmission format, but unfortunately im currently abstracting this information away during the recursion process in find_model_meshes().


to account for this, i need to actually keep track of the node tree while recursing. Leaf nodes are meshes, everything else is just a regular node. When recursing through the animation data, i can use this node tree as a reference for which mesh is actually being updated. referring back to the previous example:

    N
   / \
  m1  m2

if the animation refers to N as its output, I know to add a new Animation to all of Ns children. A more complex example:

                                  n1
                                /  |  \
                               n2  m3  m4
                              /  \
                             m1   m2  
animation with n2 applies only to m1 and m2, while an animation with n1 applies to m1-m4

## alernative data structure?

for some reason I really want to implement the data structure for this in a sort of iterator pattern.

the basic event chain is mostly the same

update -> getAnimationFrame() 
 -> query the scene animation controller
```rust
struct SceneAnimationController {
    active_animations: Vec<usize>, // keep track of top level animations that are currently running
    animations: Vec<Animation>,// or some hash map for object -> animations
}
```
each animation refers to one full animation for some model.

```rust
struct Animation {
    samplers: Vec<AnimationSampler>
}
```

animations consist of one or more animation samplers
the samplers are the parts of the animation. Each sampler will
affect one or more mesh, and they each have a unique set of times and transforms

```rust
struct AnimationSampler {
    type: AnimationType,
    meshes: Vec<usize>,
    times: Vec<usize>,
    transforms: Vec<[f32;4]>
}
```
this is where i want to implement interator. The process should be
animationController -> get active animations -> for each active animation 
-> query animation sampler for a transform

So given a certain timestamp, animation sampler needs to return the correct transform. But its not like the next
set of transforms to interpolate between is random, it proceeds from start to finish, and very frequently its just beween the same 
two transforms as the last time it was queried. (and even more frequently its that or just one step further).
To implement a binary search, or even some more clever searching method* seems wasteful.
I want to just do 
```rust
for sampler in animation.sampers.iter_mut() {
    sampler.next()
}
```
where sampler.next just checks if the given timestamp is out of bounds for the current transform set 
if it is scan further along the set of data to get the next two transforms, otherwise, the set is just stored in 
sampler.current, although we obviusly still have to interpolate.
So AnimationSampler needs to also store a current field which is something like

```rust
struct AnimationSample {
    end_time: f32, // the last time at which this sample is valid
    transform_index: usize, // why store a tuple or something if we are already storing the transforms in the parent vec
}
```


we also need to correctly apply these transforms. This is for the SimpleAnimation type, which directly 
updates the local transforms of the various mesh instances. Unlike skeletal animations, we dont need 
any inverse bind matrices. We do, however, need to walk the node tree again to calculate the new transforms each frame

for a node tree where n_ is a node and m_ is a mesh,

       n1
      /  \
     n2   m2
    /
   m1

if an animation affects, for example, n2, m1's local transform will be 
   trans(n1) * ( trans(n2) * animation_trans ) * trans(mesh)

if there was also an animation on n1, m2's transform would be
       trans(n1) * animation_n1 * trans(m2)
and for m1
    ( trans(n1) * animation_n1 ) * ( trans(n2) * animation_trans ) * trans(mesh)  

we can maybe do everything in one pass, if we store the root node for a model in SimpleAnimation


now that we have a method of obtaining the correct transform for the meshes of a model each frame,
we need to implement some logic for the animation controller. We need

1. a way to indicate to the controller that a certain animation is active on a certain instance of a model
2. a way to take a vec of transforms, and slot them into the local transform buffer at the right locations


as for the issue, we can keep it simple for now. If the '1' key is pressed, that can indicate that we want to animate the 
first instance of the first model in the scene with the first animation. 

InstanceData will be able to take a model number and an instance number, and, using the mesh indices produced by the samplers, 
change the correct slots in the overall transform buffer
