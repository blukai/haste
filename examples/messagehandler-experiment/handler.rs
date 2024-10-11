use std::marker::PhantomData;

use anyhow::Result;
use haste::parser::{Context, Visitor};
use haste::valveprotos::prost;

pub trait MessageHandler<S, M: prost::Message + Default> {
    fn handle(&self, state: &mut S, ctx: &Context, message: &M) -> Result<()>;
}

impl<S, M: prost::Message + Default, F: Fn(&mut S, &Context, &M) -> Result<()>> MessageHandler<S, M>
    for F
{
    fn handle(&self, state: &mut S, ctx: &Context, message: &M) -> Result<()> {
        self(state, ctx, message)
    }
}

trait Handler<S> {
    fn handle(&mut self, state: &mut S, ctx: &Context, packet_type: u32, data: &[u8])
    -> Result<()>;
}

struct HandlerImpl<S, M: prost::Message + Default, H: MessageHandler<S, M>> {
    id: u32,
    handler: Box<H>,
    _phantom1: PhantomData<S>,
    _phantom2: PhantomData<M>,
}

impl<S, M: prost::Message + Default, H: MessageHandler<S, M>> HandlerImpl<S, M, H> {
    pub fn new(id: u32, handler: H) -> Box<Self> {
        Box::new(Self {
            id,
            handler: Box::new(handler),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        })
    }
}

impl<S, M: prost::Message + Default, H: MessageHandler<S, M>> Handler<S> for HandlerImpl<S, M, H> {
    fn handle(
        &mut self,
        state: &mut S,
        ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> Result<()> {
        if packet_type != self.id {
            return Ok(());
        }

        let msg = M::decode(data)?;

        self.handler.handle(state, ctx, &msg)
    }
}

pub struct HandlerVisitor<S> {
    state: S,
    handlers: Vec<Box<dyn Handler<S>>>,
}

impl<S: 'static> HandlerVisitor<S> {
    pub fn with_state(state: S) -> Self {
        Self {
            state,
            handlers: vec![],
        }
    }

    pub fn with<M: prost::Message + Default + 'static, H: MessageHandler<S, M> + 'static>(
        mut self,
        id: u32,
        handler: H,
    ) -> Self {
        self.handlers.push(HandlerImpl::new(id, handler));

        self
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<S> Visitor for &mut HandlerVisitor<S> {
    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> Result<()> {
        for handler in self.handlers.iter_mut() {
            handler.handle(&mut self.state, ctx, packet_type, data)?
        }

        Ok(())
    }
}
