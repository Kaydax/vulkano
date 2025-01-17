// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::check_errors;
use crate::instance::loader;
use crate::instance::loader::LoadingError;
use crate::Error;
use crate::OomError;
use crate::Version;
use std::error;
use std::ffi::CStr;
use std::fmt;
use std::ptr;

/// Queries the list of layers that are available when creating an instance.
///
/// On success, this function returns an iterator that produces
/// [`LayerProperties`](crate::instance::LayerProperties) objects. In order to enable a layer, you need
/// to pass its name (returned by `LayerProperties::name()`) when creating the
/// [`Instance`](crate::instance::Instance).
///
/// This function returns an error if it failed to load the Vulkan library.
///
/// > **Note**: It is possible that one of the layers enumerated here is no longer available when
/// > you create the `Instance`. This will lead to an error when calling `Instance::new`. The
/// > author isn't aware of any situation where this would happen, but it is theoretically possible
/// > according to the specifications.
///
/// # Example
///
/// ```no_run
/// use vulkano::instance;
///
/// for layer in instance::layers_list().unwrap() {
///     println!("Available layer: {}", layer.name());
/// }
/// ```
pub fn layers_list() -> Result<impl ExactSizeIterator<Item = LayerProperties>, LayersListError> {
    layers_list_from_loader(loader::auto_loader()?)
}

/// Same as `layers_list()`, but allows specifying a loader.
pub fn layers_list_from_loader<L>(
    ptrs: &loader::FunctionPointers<L>,
) -> Result<impl ExactSizeIterator<Item = LayerProperties>, LayersListError>
where
    L: loader::Loader,
{
    unsafe {
        let fns = ptrs.fns();

        let mut num = 0;
        check_errors(
            fns.v1_0
                .enumerate_instance_layer_properties(&mut num, ptr::null_mut()),
        )?;

        let mut layers: Vec<ash::vk::LayerProperties> = Vec::with_capacity(num as usize);
        check_errors({
            fns.v1_0
                .enumerate_instance_layer_properties(&mut num, layers.as_mut_ptr())
        })?;
        layers.set_len(num as usize);

        Ok(layers.into_iter().map(|p| LayerProperties { props: p }))
    }
}

/// Properties of a layer.
#[derive(Clone)]
pub struct LayerProperties {
    props: ash::vk::LayerProperties,
}

impl LayerProperties {
    /// Returns the name of the layer.
    ///
    /// If you want to enable this layer on an instance, you need to pass this value to
    /// `Instance::new`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vulkano::instance;
    ///
    /// for layer in instance::layers_list().unwrap() {
    ///     println!("Layer name: {}", layer.name());
    /// }
    /// ```
    #[inline]
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.props.layer_name.as_ptr())
                .to_str()
                .unwrap()
        }
    }

    /// Returns a description of the layer.
    ///
    /// This description is chosen by the layer itself.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vulkano::instance;
    ///
    /// for layer in instance::layers_list().unwrap() {
    ///     println!("Layer description: {}", layer.description());
    /// }
    /// ```
    #[inline]
    pub fn description(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.props.description.as_ptr())
                .to_str()
                .unwrap()
        }
    }

    /// Returns the version of Vulkan supported by this layer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vulkano::instance;
    /// use vulkano::instance::Version;
    ///
    /// for layer in instance::layers_list().unwrap() {
    ///     if layer.vulkan_version() >= Version::major_minor(2, 0) {
    ///         println!("Layer {} requires Vulkan 2.0", layer.name());
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn vulkan_version(&self) -> Version {
        Version::from(self.props.spec_version)
    }

    /// Returns an implementation-specific version number for this layer.
    ///
    /// The number is chosen by the layer itself. It can be used for bug reports for example.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vulkano::instance;
    ///
    /// for layer in instance::layers_list().unwrap() {
    ///     println!("Layer {} - Version: {}", layer.name(), layer.implementation_version());
    /// }
    /// ```
    #[inline]
    pub fn implementation_version(&self) -> u32 {
        self.props.implementation_version
    }
}

/// Error that can happen when loading the list of layers.
#[derive(Clone, Debug)]
pub enum LayersListError {
    /// Failed to load the Vulkan shared library.
    LoadingError(LoadingError),
    /// Not enough memory.
    OomError(OomError),
}

impl error::Error for LayersListError {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            LayersListError::LoadingError(ref err) => Some(err),
            LayersListError::OomError(ref err) => Some(err),
        }
    }
}

impl fmt::Display for LayersListError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "{}",
            match *self {
                LayersListError::LoadingError(_) => "failed to load the Vulkan shared library",
                LayersListError::OomError(_) => "not enough memory available",
            }
        )
    }
}

impl From<OomError> for LayersListError {
    #[inline]
    fn from(err: OomError) -> LayersListError {
        LayersListError::OomError(err)
    }
}

impl From<LoadingError> for LayersListError {
    #[inline]
    fn from(err: LoadingError) -> LayersListError {
        LayersListError::LoadingError(err)
    }
}

impl From<Error> for LayersListError {
    #[inline]
    fn from(err: Error) -> LayersListError {
        match err {
            err @ Error::OutOfHostMemory => LayersListError::OomError(OomError::from(err)),
            err @ Error::OutOfDeviceMemory => LayersListError::OomError(OomError::from(err)),
            _ => panic!("unexpected error: {:?}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::instance;

    #[test]
    fn layers_list() {
        let mut list = match instance::layers_list() {
            Ok(l) => l,
            Err(_) => return,
        };

        while let Some(_) = list.next() {}
    }
}
