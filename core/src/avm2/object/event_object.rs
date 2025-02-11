//! Object representation for events

use crate::avm2::activation::Activation;
use crate::avm2::events::Event;
use crate::avm2::object::script_object::ScriptObjectData;
use crate::avm2::object::{ClassObject, Object, ObjectPtr, ScriptObject, TObject};
use crate::avm2::value::Value;
use crate::avm2::Error;
use crate::context::UpdateContext;
use crate::display_object::TDisplayObject;
use crate::display_object::{DisplayObject, InteractiveObject, TInteractiveObject};
use crate::events::{KeyCode, MouseButton};
use crate::string::AvmString;
use gc_arena::barrier::unlock;
use gc_arena::{lock::RefLock, Collect, Gc, GcWeak, Mutation};
use std::cell::{Ref, RefMut};
use std::fmt::Debug;

/// A class instance allocator that allocates Event objects.
pub fn event_allocator<'gc>(
    class: ClassObject<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Object<'gc>, Error<'gc>> {
    let base = ScriptObjectData::new(class);

    Ok(EventObject(Gc::new(
        activation.gc(),
        EventObjectData {
            base,
            event: RefLock::new(Event::new(activation.strings().empty())),
        },
    ))
    .into())
}

#[derive(Clone, Collect, Copy)]
#[collect(no_drop)]
pub struct EventObject<'gc>(pub Gc<'gc, EventObjectData<'gc>>);

#[derive(Clone, Collect, Copy, Debug)]
#[collect(no_drop)]
pub struct EventObjectWeak<'gc>(pub GcWeak<'gc, EventObjectData<'gc>>);

#[derive(Clone, Collect)]
#[collect(no_drop)]
#[repr(C, align(8))]
pub struct EventObjectData<'gc> {
    /// Base script object
    base: ScriptObjectData<'gc>,

    /// The event this object holds.
    event: RefLock<Event<'gc>>,
}

const _: () = assert!(std::mem::offset_of!(EventObjectData, base) == 0);
const _: () =
    assert!(std::mem::align_of::<EventObjectData>() == std::mem::align_of::<ScriptObjectData>());

impl<'gc> EventObject<'gc> {
    /// Create a bare Event instance while skipping the usual `construct()` pipeline.
    /// It's just slightly faster and doesn't require an Activation.
    /// This is equivalent to
    /// classes.event.construct(activation, &[event_type, false, false])
    pub fn bare_default_event<S>(
        context: &mut UpdateContext<'gc>,
        event_type: S,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        Self::bare_event(context, event_type, false, false)
    }

    /// Create a bare Event instance while skipping the usual `construct()` pipeline.
    /// It's just slightly faster and doesn't require an Activation.
    /// Note that if you need an Event subclass, you need to construct it via .construct().
    pub fn bare_event<S>(
        context: &mut UpdateContext<'gc>,
        event_type: S,
        bubbles: bool,
        cancelable: bool,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        let class = context.avm2.classes().event;
        let base = ScriptObjectData::new(class);

        let mut event = Event::new(event_type);
        event.set_bubbles(bubbles);
        event.set_cancelable(cancelable);

        EventObject(Gc::new(
            context.gc(),
            EventObjectData {
                base,
                event: RefLock::new(event),
            },
        ))
    }

    #[inline]
    pub fn from_class_and_args(
        activation: &mut Activation<'_, 'gc>,
        class: ClassObject<'gc>,
        args: &[Value<'gc>],
    ) -> EventObject<'gc> {
        // We don't expect Event classes to error in their constructors or to
        // return anything other than an EventObject
        class
            .construct(activation, args)
            .unwrap()
            .as_object()
            .unwrap()
            .as_event_object()
            .unwrap()
    }

    pub fn mouse_event<S>(
        activation: &mut Activation<'_, 'gc>,
        event_type: S,
        target: DisplayObject<'gc>,
        related_object: Option<InteractiveObject<'gc>>,
        delta: i32,
        bubbles: bool,
        button: MouseButton,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        let local = target.local_mouse_position(activation.context);

        let event_type: AvmString<'gc> = event_type.into();

        let mouse_event_cls = activation.avm2().classes().mouseevent;
        Self::from_class_and_args(
            activation,
            mouse_event_cls,
            &[
                event_type.into(),
                // bubbles
                bubbles.into(),
                // cancellable
                false.into(),
                // localX
                local.x.to_pixels().into(),
                // localY
                local.y.to_pixels().into(),
                // relatedObject
                related_object
                    .map(|o| o.as_displayobject().object2())
                    .unwrap_or(Value::Null),
                // ctrlKey
                activation
                    .context
                    .input
                    .is_key_down(KeyCode::CONTROL)
                    .into(),
                // altKey
                activation.context.input.is_key_down(KeyCode::ALT).into(),
                // shiftKey
                activation.context.input.is_key_down(KeyCode::SHIFT).into(),
                // buttonDown
                activation.context.input.is_key_down(button.into()).into(),
                // delta
                delta.into(),
            ],
        )
    }

    pub fn mouse_event_down(
        activation: &mut Activation<'_, 'gc>,
        target: DisplayObject<'gc>,
        button: MouseButton,
    ) -> EventObject<'gc> {
        Self::mouse_event(
            activation,
            match button {
                MouseButton::Left => "mouseDown",
                MouseButton::Right => "rightMouseDown",
                MouseButton::Middle => "middleMouseDown",
                MouseButton::Unknown => unreachable!(),
            },
            target,
            None,
            0,
            true,
            button,
        )
    }

    pub fn mouse_event_up(
        activation: &mut Activation<'_, 'gc>,
        target: DisplayObject<'gc>,
        button: MouseButton,
    ) -> EventObject<'gc> {
        Self::mouse_event(
            activation,
            match button {
                MouseButton::Left => "mouseUp",
                MouseButton::Right => "rightMouseUp",
                MouseButton::Middle => "middleMouseUp",
                MouseButton::Unknown => unreachable!(),
            },
            target,
            None,
            0,
            true,
            button,
        )
    }

    pub fn mouse_event_click(
        activation: &mut Activation<'_, 'gc>,
        target: DisplayObject<'gc>,
        button: MouseButton,
    ) -> EventObject<'gc> {
        Self::mouse_event(
            activation,
            match button {
                MouseButton::Left => "click",
                MouseButton::Right => "rightClick",
                MouseButton::Middle => "middleClick",
                MouseButton::Unknown => unreachable!(),
            },
            target,
            None,
            0,
            true,
            button,
        )
    }

    pub fn text_event<S>(
        activation: &mut Activation<'_, 'gc>,
        event_type: S,
        text: AvmString<'gc>,
        bubbles: bool,
        cancelable: bool,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        let event_type: AvmString<'gc> = event_type.into();

        let text_event_cls = activation.avm2().classes().textevent;
        Self::from_class_and_args(
            activation,
            text_event_cls,
            &[
                event_type.into(),
                // bubbles
                bubbles.into(),
                // cancelable
                cancelable.into(),
                // text
                text.into(),
            ],
        )
    }

    pub fn net_status_event(
        activation: &mut Activation<'_, 'gc>,
        info: Vec<(impl Into<AvmString<'gc>>, impl Into<AvmString<'gc>>)>,
    ) -> EventObject<'gc> {
        let info_object = ScriptObject::new_object(activation);
        for (key, value) in info {
            info_object
                .set_string_property_local(key.into(), Value::String(value.into()), activation)
                .unwrap();
        }

        let net_status_cls = activation.avm2().classes().netstatusevent;
        Self::from_class_and_args(
            activation,
            net_status_cls,
            &[
                "netStatus".into(),
                //bubbles
                false.into(),
                //cancelable
                false.into(),
                info_object.into(),
            ],
        )
    }

    pub fn progress_event<S>(
        activation: &mut Activation<'_, 'gc>,
        event_type: S,
        bytes_loaded: usize,
        bytes_total: usize,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        let event_type: AvmString<'gc> = event_type.into();

        let progress_event_cls = activation.avm2().classes().progressevent;
        Self::from_class_and_args(
            activation,
            progress_event_cls,
            &[
                event_type.into(),
                // bubbles
                false.into(),
                // cancelable
                false.into(),
                // bytesLoaded
                (bytes_loaded as f64).into(),
                // bytesToal
                (bytes_total as f64).into(),
            ],
        )
    }

    pub fn focus_event<S>(
        activation: &mut Activation<'_, 'gc>,
        event_type: S,
        cancelable: bool,
        related_object: Option<InteractiveObject<'gc>>,
        key_code: u32,
    ) -> EventObject<'gc>
    where
        S: Into<AvmString<'gc>>,
    {
        let event_type: AvmString<'gc> = event_type.into();
        let shift_key = activation.context.input.is_key_down(KeyCode::SHIFT);

        let focus_event_cls = activation.avm2().classes().focusevent;
        Self::from_class_and_args(
            activation,
            focus_event_cls,
            &[
                event_type.into(),
                true.into(),
                cancelable.into(),
                related_object
                    .map(|o| o.as_displayobject().object2())
                    .unwrap_or(Value::Null),
                shift_key.into(),
                key_code.into(),
                "none".into(), // TODO implement direction
            ],
        )
    }

    pub fn io_error_event(
        activation: &mut Activation<'_, 'gc>,
        error_msg: AvmString<'gc>,
        error_code: u32,
    ) -> EventObject<'gc> {
        let io_error_event_cls = activation.avm2().classes().ioerrorevent;
        Self::from_class_and_args(
            activation,
            io_error_event_cls,
            &[
                "ioError".into(),
                false.into(),
                false.into(),
                error_msg.into(),
                error_code.into(),
            ],
        )
    }

    pub fn http_status_event(
        activation: &mut Activation<'_, 'gc>,
        status: u16,
        redirected: bool,
    ) -> EventObject<'gc> {
        let http_status_event_cls = activation.avm2().classes().httpstatusevent;
        Self::from_class_and_args(
            activation,
            http_status_event_cls,
            &[
                "httpStatus".into(),
                false.into(),
                false.into(),
                status.into(),
                redirected.into(),
            ],
        )
    }

    pub fn event(&self) -> Ref<Event<'gc>> {
        self.0.event.borrow()
    }

    pub fn event_mut(&self, mc: &Mutation<'gc>) -> RefMut<Event<'gc>> {
        unlock!(Gc::write(mc, self.0), EventObjectData, event).borrow_mut()
    }
}

impl<'gc> TObject<'gc> for EventObject<'gc> {
    fn gc_base(&self) -> Gc<'gc, ScriptObjectData<'gc>> {
        // SAFETY: Object data is repr(C), and a compile-time assert ensures
        // that the ScriptObjectData stays at offset 0 of the struct- so the
        // layouts are compatible

        unsafe { Gc::cast(self.0) }
    }

    fn as_ptr(&self) -> *const ObjectPtr {
        Gc::as_ptr(self.0) as *const ObjectPtr
    }

    fn as_event_object(self) -> Option<EventObject<'gc>> {
        Some(self)
    }

    fn as_event(&self) -> Option<Ref<Event<'gc>>> {
        Some(self.0.event.borrow())
    }

    fn as_event_mut(&self, mc: &Mutation<'gc>) -> Option<RefMut<Event<'gc>>> {
        Some(unlock!(Gc::write(mc, self.0), EventObjectData, event).borrow_mut())
    }
}

impl Debug for EventObject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EventObject")
            .field("type", &self.0.event.borrow().event_type())
            .field("class", &self.base().debug_class_name())
            .field("ptr", &Gc::as_ptr(self.0))
            .finish()
    }
}
