# falling_petals

This code renders thousands of marigold petals falling and blowing around.  It was written to create
a visualization that was projected onto a building for a Day of the Dead event.  If you have
[ffmpeg](https://ffmpeg.org/) installed and available on your computer's PATH, it can also render
each frame to an off-screen buffer (with the desired resolution) and pipe it to ffmpeg to be encoded
into a video file.  A sample video of its output is available on
[YouTube](https://youtu.be/ap0XRhDKJp4).

When the program runs, it will look for a `config.toml` file from which to load various settings.
If this config file does not exist, it will generate a default one (containing comments explaining
what the various settings are) and exit.  You can then modify the config file if desired, and run
the program again.

## Caveats

This is a personal project that I used as a way to learn Rust and modern GPU programming.  My only
experience with Rust prior to this project came from reading the Rust Book and following along
through a few other Rust tutorials (e.g. the [WGPU Tutorial](https://sotrh.github.io/learn-wgpu/)
and [Learn Rust With Entirely Too Many Linked
Lists](https://rust-unofficial.github.io/too-many-lists/)).  I did have prior experience with OpenGL
programming, but it was from a long time ago when GPUs were still using the fixed function pipeline
(before shader languages existed).  As such, this code may not do everything in the "best" way
possible.

## Camera movement controls

The camera is intended to remain stationary while generating the final visualization.  However, I
included controls to move it around as that can be very helpful while debugging and adjusting how
things are rendered.  The controls are similar to those used in first-person shooter games, and are
as follows:

- **Right-click:** Toggle mouselook / mouse capture.  By default, mouselook starts disabled to avoid
  accidentally moving the camera while exporting a video.
- **Mouse movement:** When mouselook is enabled, controls the pan (or yaw) and pitch angles of the
  camera.  The roll angle is not currently controllable (the camera's up vector is locked to the
  global +y axis).
- **[W]:** Move forward (within the current x/z-plane).
- **[S]:** Move backward (within the current x/z-plane).
- **[A]:** Slide left (within the current x/z-plane).
- **[D]:** Slide right (within the current x/z-plane).
- **[Spacebar]:** Slide up in the +z direction.
- **[C]:** Slide down in the -z direction.
- **[Esc]:** Exit the program (closes and finishes any video export first).

## Implementation details

- ### Simulation and rendering steps are tied together

  For simplicity, one simulation step (physics update) is taken for each frame that is rendered.
  This approach is appropriate in this case because my primary goal is to export to video and there
  are not any complex interactions between the user and the simulation state that depend on timing.
  For game programming, it's generally a bad idea since it can make your game behave differently
  when the frame rate changes.  In that case, it's better to have a fixed-rate game state update and
  render as many frames as possible between those updates by interpolating the state.

- ### No lighting

  While it wouldn't be particularly hard to add diffuse lighting, it has not been a priority.  I
  intended this to be projected onto a building at night, where the shape and textures of the
  building, the location of the projector, and other lighting conditions in the area will all affect
  what the projection looks like.  As such, I doubted that adding diffuse lighting would make a
  significantly noticeable difference, and figured that it might be best to just project the colors
  as vibrantly as possible for visibility.  So I simply render the actual texture colors without any
  modification to simulate lighting.

- ### Z-ordering for alpha blending

  The petals are sorted by their global Z coordinates each frame so that they will render from back
  to front in the simulation volume.  This is necessary to get correct alpha blending for pixels
  that are partially transparent (usually those around the edges of the petal).  However, there is
  an uncommon case where a petal close to and behind another petal can be oriented such that part
  of the petal in back extends in front of the petal in front (possibly but not necessarily
  intersecting it).  In this case, the alpha blending will be incorrect for the part of the back
  petal that extends in front of the front petal, causing a black outline to appear around the edges
  of that part of the petal.  Given that this does not happen often and likely won't be very
  noticeable when projected on a building, I have decided not to fix it.  Correctly rendering
  intersecting transparent objects is generally a difficult problem.  Probably an easier way to
  alleviate this problem (if desired) would be to enforce a minimum separation between petals based
  on their size.

  Note that since the petals are ordered based on their global Z coordinates and not according to
  their depth from the perspective of the camera, the alpha blending will be incorrect if you move
  the camera to the back of the simulation area and turn it around to view the petals from the other
  direction.  In this case, you will see the black outlines around nearly all the petals as they
  pass in front of other petals.  Since I intend the camera to remain stationary and primarily use
  it's movement controls for debugging, I do not view this as a problem.

- ### Discard pixels that are close to fully transparent

  The WGSL shader code discards pixels that are fully transparent (or very close to that).  If I did
  not do this, then the problem mentioned above that can cause a black outline to appear around the
  edges of a petal would instead cause a big black rectangle, which would be much more noticeable
  when it happened.  This is because the petals are rendered as a rectangular fan of triangles and
  the depth buffer would get updated for all the invisible pixels in that rectangle, thus preventing
  anything behind it from getting drawn in the future.  By discarding these pixels, the depth buffer
  does not get updated in those locations and instead only gets updated for the partially
  transparent pixels around the edges of the petal.

- ### Petal bend

  If the petals are rendered as a perfectly flat quad, they tend to disappear almost completely when
  they are rotated such that they are viewed edge-on.  This doesn't look very good / realistic,
  since real petals have depth and are not perfectly flat.  Thus, I render each petal as a fan of 8
  triangles forming a rectangle around the central point and displace the points around the edges
  so that they are not perfectly co-planar (this displacement can be adjusted in the config file).
  This makes the petals look much better when they are viewed edge-on, as they usually just look
  thin instead of disappearing completely.

- ### Anti-aliasing

  I currently have not explored doing any anti-aliasing.  Similar to lighting, I doubt that it
  would make a very noticeable difference when the visualization is projected onto a building.
  However, it probably could help improve the appearance of the petals when viewed edge-on.

- ### Fade close to near / far planes

  When petals intersect the near or far planes, they get clipped.  This looks strange and also
  causes petals entering or exiting the view frustum through those planes to suddenly appear or
  disappear.  To alleviate this, the WGSL shader code adjusts the transparency of pixels that are
  close to either the near or far planes (in normalized device coordinates) so that they become more
  and more transparent the closer they get.  This gives a much nicer fading in/out effect for petals
  passing through those planes.

- ### Uniform buffer index packing

  At the beginning of the program, information about each petal variant (which portion of which
  texture contains the petal image) is transferred into a GPU buffer.  Each of the petals spawned
  is randomly assigned an index into those variants, which determines what petal image will be used
  for that petal.  For each frame rendered, the code passes information about each petal's pose and
  its variant index into the shader code---all ordered from back to front so that the alpha blending
  works correctly.  The pose information is transferred via a vertex buffer, and the variant index
  information is transferred in via a uniform buffer (which may not have been the best choice, but
  that's how it is for now).

  Uniform buffers have an 16-byte alignment constraint, and their maximum size is also fairly
  limited (65536 bytes on my GPU).  I originally passed the variant index data in as u32s aligned to
  16 byte boundaries (thus having 12 bytes of padding between each 4-byte u32) so that they could be
  indexed directly from the uniform buffer.  However, with a maximum uniform buffer size of 65 kB,
  this limited me to a maximum of 4096 petals since each index was eating up 16 bytes of space.  To
  allow for more petals than that, I changed the code to densely pack u32 indexes into the uniform
  buffer and I now take care of sub-indexing each u32 out on the shader side of the code (by
  interpreting the buffer as an array of vec4 of u32).  This adds a small amount of complexity to
  the shader code, but allows me to simulate up to 16384 petals at once.  To avoid requiring
  n_petals to be a multiple of 4, I pad the end of the index data array with zeros to get its size
  aligned on a 16-byte boundary.

  A storage buffer might have been easier to use (they have looser constraints on size and
  alignment), but they are generally slower than uniform buffers.  Passing the indexes in via a
  vertex buffer might be a better option, but I have not explored that yet.

- ### Petals move in lock-step and each petal has a constant rotation

  Originally, I planned on including randomness in how each petal moves relative to the others and
  in how it rotates over time.  However, I started with the simpler option of giving all the petals
  a constant rotation (randomly chosen at program start) and having them all move together, and I
  was more satisfied with the results than I thought I would be (it's not as visually obvious as I
  thought it might be, likely because of the perspective projection and the randomized rotation
  directions).

- ### Video export x-resolution must be a multilpe of 64

  When copying data out of a texture to a buffer, WGPU requires that the data for each row be
  aligned to wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, which is currently defined to be 256.  With 4 bytes
  of information per pixel, this means that the x resolution must be a multiple of 64 to avoid
  needing any padding.  Most current standard video resolutions do have an x-resolution that is a
  muliple of 64, so it doesn't seem like that restriction is very limiting.  As such, I have not
  considered it enough of a priority to write the code needed to appropriately pad the off-screen
  buffer size and remove the padding at the end of each row when copying data out of the off-screen
  buffer and piping it to ffmpeg.  Instead, I just assume that no padding is necessary and explian
  that requiement in the comments in the default config file.

## Petal textures

I created the [PetalsArranged.png](falling_petals/res/PetalsArranged.png) texture included in this
repository by:

- Taking images of actual marigold petals against a dark background.
- Using the Fuzzy Select tool in [GIMP](https://www.gimp.org/) to remove the background.
- Manually cleaning out spots / speckles missed by fuzzy select.
- Arranging the petals within a 128x128 grid, with petals of similar size (e.g. 3 grid cells by 1
  grid cell) stacked together in the same column.  This makes it easier to compute the texture
  coordinates to slice out individual petals.

Other images can also be used as textures by adding the information about the file path and what
parts of it constitute individual petals to the config.toml file.
