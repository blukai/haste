use std::marker::PhantomData;

use haste::{entities::UpdateType, fieldvalue::FieldValue, fxhash};

use crate::{
    entities,
    parser::{self, Context, Visitor},
};

pub trait PlayerState {
    fn set_player(&mut self, player: Player);
    fn get_player(&self, slot: &i32) -> Option<&Player>;
}

pub trait MessageHandler<S: PlayerState, M: prost::Message + Default> {
    fn handle(&self, state: &mut S, ctx: &Context, message: &M) -> parser::Result<()>;
}

impl<
        S: PlayerState,
        M: prost::Message + Default,
        F: Fn(&mut S, &Context, &M) -> parser::Result<()>,
    > MessageHandler<S, M> for F
{
    fn handle(&self, state: &mut S, ctx: &Context, message: &M) -> parser::Result<()> {
        self(state, ctx, message)
    }
}

trait Handler<S> {
    fn handle(
        &mut self,
        state: &mut S,
        ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> parser::Result<()>;
}

struct HandlerImpl<S: PlayerState, M: prost::Message + Default, H: MessageHandler<S, M>> {
    id: u32,
    handler: Box<H>,
    _phantom1: PhantomData<S>,
    _phantom2: PhantomData<M>,
}

impl<S: PlayerState, M: prost::Message + Default, H: MessageHandler<S, M>> HandlerImpl<S, M, H> {
    pub fn new(id: u32, handler: H) -> Box<Self> {
        Box::new(Self {
            id,
            handler: Box::new(handler),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        })
    }
}

impl<S: PlayerState, M: prost::Message + Default, H: MessageHandler<S, M>> Handler<S>
    for HandlerImpl<S, M, H>
{
    fn handle(
        &mut self,
        state: &mut S,
        ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> parser::Result<()> {
        if packet_type != self.id {
            return Ok(());
        }

        let msg = M::decode(data)?;

        self.handler.handle(state, ctx, &msg)
    }
}

pub struct Player {
    pub slot: i32,
    pub team_id: u8,
    pub name: String,
}

pub struct HandlerVisitor<S: PlayerState> {
    state: S,
    handlers: Vec<Box<dyn Handler<S>>>,
}

impl<S: PlayerState + 'static> HandlerVisitor<S> {
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

impl<S: PlayerState> Visitor for &mut HandlerVisitor<S> {
    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> parser::Result<()> {
        for handler in self.handlers.iter_mut() {
            handler.handle(&mut self.state, ctx, packet_type, data)?
        }

        Ok(())
    }

    fn on_entity(
        &mut self,
        ctx: &Context,
        update_flags: usize,
        update_type: crate::entities::UpdateType,
        // TODO: include updated fields (list of field paths?)
        entity: &crate::entities::Entity,
    ) -> parser::Result<()> {
        if let UpdateType::EnterPVS = update_type {
            return Ok(());
        }

        let ser_key: u64 = fxhash::hash_bytes(b"CCitadelPlayerController");

        let ser = entity.serializer().serializer_name.hash;

        if ser != ser_key {
            return Ok(());
        }

        let team_key: u64 = entities::make_field_key(&["m_iTeamNum"]);
        let teamname_key: u64 = entities::make_field_key(&["m_szTeamname"]);
        let playername_key: u64 = entities::make_field_key(&["m_iszPlayerName"]);
        let playerslot_key: u64 = entities::make_field_key(&["m_unLobbyPlayerSlot"]);

        if let (
            Some(FieldValue::U8(team_id)),
            Some(FieldValue::String(name)),
            Some(FieldValue::U32(slot)),
        ) = (
            entity.get_value(&team_key),
            entity.get_value(&playername_key),
            entity.get_value(&playerslot_key),
        ) {
            self.state.set_player(Player {
                slot: *slot as i32,
                team_id: *team_id,
                name: name.to_string(),
            });
        }
        Ok(())
    }
}
