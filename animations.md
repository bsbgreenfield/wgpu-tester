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

theoretically both 1 and 2 could be async, and creating the new lt buffer can happen only once 
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

