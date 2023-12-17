use crate::data_source::DataSource;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::{egui, emath, epaint};
use hashbrown::HashMap;
use haste_dota2::{dota2_protos::CDemoFileInfo, entities::Entity};
use std::{fs::File, io, sync::Arc};

// NOTE: egui does not support zooming or panning, look at
// https://sourcegraph.com/github.com/gzp79/shine/-/blob/ui/src/node_graph/zoom_pan.rs?L7:34-7:46

#[derive(Debug)]
enum ParserMessage {}

#[derive(Debug)]
enum UiMessage {
    ConstructionOk,
    ConstructionErr(String),
    FileInfo(haste_dota2::parser::Result<CDemoFileInfo>),
    Entity(Entity),
}

pub(crate) struct ReplayView {
    file_name: String,

    ui_receiver: Receiver<UiMessage>,
    parser_sender: Sender<ParserMessage>,

    file_info: Option<CDemoFileInfo>,
    entities: HashMap<i32, Entity, haste_dota2::nohash::NoHashHasherBuilder<i32>>,
}

impl ReplayView {
    pub(crate) fn new(data_source: DataSource) -> Self {
        let file_name = data_source.file_name().to_owned();

        let (ui_sender, ui_receiver) = unbounded::<UiMessage>();
        let (parser_sender, parser_receiver) = unbounded::<ParserMessage>();

        std::thread::spawn(move || {
            struct MyVisitor {
                ui_sender: Sender<UiMessage>,
            };
            impl haste_dota2::parser::Visitor for MyVisitor {
                fn visit_entity(
                    &self,
                    update_flags: usize,
                    update_type: haste_dota2::entities::UpdateType,
                    entity: &haste_dota2::entities::Entity,
                ) -> haste_dota2::parser::Result<()> {
                    // TODO: do not clone each entity, but rather send a batch
                    // after completion of parser request
                    self.ui_sender
                        .send(UiMessage::Entity(entity.clone()))
                        .map_err(|err| err.into())
                }
            }

            trait ReadSeek: io::Read + io::Seek {}
            impl<T> ReadSeek for T where T: io::Read + io::Seek {}
            fn into_read_seek(
                data_source: DataSource,
            ) -> std::result::Result<Box<dyn ReadSeek>, Box<dyn std::error::Error>> {
                match data_source {
                    DataSource::FilePath(file_path) => Ok(Box::new(File::open(file_path)?)),
                    DataSource::FileContents { bytes, .. } => Ok(Box::new(io::Cursor::new(bytes))),
                }
            }

            match into_read_seek(data_source)
                .map(io::BufReader::new)
                .and_then(|buf_reader| {
                    haste_dota2::parser::Parser::from_reader(
                        buf_reader,
                        MyVisitor {
                            ui_sender: ui_sender.clone(),
                        },
                    )
                }) {
                Err(err) => {
                    ui_sender
                        .send(UiMessage::ConstructionErr(err.to_string()))
                        .ok();
                }
                Ok(mut parser) => {
                    ui_sender.send(UiMessage::ConstructionOk).ok();
                    ui_sender
                        .send(UiMessage::FileInfo(parser.file_info().cloned()))
                        .ok();
                    parser.parse_to_tick(42);
                }
            }
        });

        Self {
            file_name,

            ui_receiver,
            parser_sender,

            file_info: None,
            entities: Default::default(),
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        // // TODO: move out into a separate func
        // egui::CentralPanel::default()
        //     .frame(ReplayView::frame())
        //     .show_inside(ui, |ui| {
        //         egui::ScrollArea::vertical()
        //             .auto_shrink([false, false])
        //             .drag_to_scroll(false)
        //             .show(ui, |ui| {
        //                 self.entities.values().for_each(|entity| {
        //                     let id = ui.id().with(entity.index);
        //                     egui::collapsing_header::CollapsingState::load_with_default_open(
        //                         ui.ctx(),
        //                         id,
        //                         false,
        //                     )
        //                     .show_header(ui, |ui| {
        //                         ui.label(entity.flattened_serializer.serializer_name.to_string());
        //                     })
        //                     .body(|ui| {
        //                         Self::show_fields(
        //                             ui,
        //                             &entity.flattened_serializer.fields,
        //                             &entity.field_values,
        //                             haste_dota2::fieldpath::FieldPath::default(),
        //                         );
        //                     });
        //                 });
        //             });
        //     });

        self.entities.values().for_each(|entity| {
            egui::Window::new(entity.index.to_string())
                .open(&mut true)
                .title_bar(false)
                .show(ui.ctx(), |ui| {
                    let id = ui.id().with(entity.index);
                    egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        id,
                        true,
                    )
                    .show_header(ui, |ui| {
                        ui.label(entity.flattened_serializer.serializer_name.to_string());
                    })
                    .body(|ui| {
                        Self::show_fields(
                            ui,
                            &entity.flattened_serializer.fields,
                            &entity.field_values,
                            haste_dota2::fieldpath::FieldPath::default(),
                        );
                    });
                });
        });
    }

    pub(crate) fn update(&mut self) {
        while let Some(message) = self.ui_receiver.try_recv().ok() {
            match message {
                UiMessage::FileInfo(file_info) => match file_info {
                    Ok(file_info) => {
                        self.file_info = Some(file_info);
                    }
                    Err(err) => {
                        eprintln!("unhandled file info err: {:?}", err);
                    }
                },
                UiMessage::Entity(entity) => {
                    self.entities.insert(entity.index, entity);
                }
                _ => {
                    eprintln!("unhandled ui message: {:?}", message);
                }
            }
        }
    }

    // design tokens

    fn background() -> epaint::Color32 {
        // TODO: get from visuals.panel_fill
        epaint::Color32::from_rgb(13, 16, 17)
    }

    fn inner_margin() -> egui::Margin {
        egui::Margin::same(0.0)
    }

    fn frame() -> egui::Frame {
        egui::Frame {
            fill: Self::background(),
            inner_margin: Self::inner_margin(),
            ..Default::default()
        }
    }

    // temp stuff

    fn show_fields(
        ui: &mut egui::Ui,
        fields: &Vec<Arc<haste_dota2::flattenedserializers::FlattenedSerializerField>>,
        field_values: &HashMap<
            u64,
            haste_dota2::fieldvalue::FieldValue,
            haste_dota2::nohash::NoHashHasherBuilder<u64>,
        >,
        field_path: haste_dota2::fieldpath::FieldPath,
    ) {
        let mut field_path = field_path;

        fields.iter().enumerate().for_each(|(i, field)| {
            field_path.data[field_path.position] = i as i32;
            let field_key = unsafe { field_path.hash_unchecked() };
            if let Some(field_value) = field_values.get(&field_key) {
                match field.field_serializer.as_ref() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = emath::Vec2::ZERO;

                            ui.label(field.var_name.to_string());
                            ui.label(": ");

                            ui.scope(|ui| {
                                ui.set_enabled(false);
                                ui.label(field.var_type.to_string());
                            });

                            ui.strong(format!(" {:?}", field_value));
                        });
                    }
                    Some(field_serializer) => {
                        let has_field_values = {
                            let mut field_path = field_path.clone();
                            field_path.position += 1;
                            field_serializer
                                .fields
                                .iter()
                                .enumerate()
                                .any(|(i, _field)| {
                                    field_path.data[field_path.position] = i as i32;
                                    let field_key = unsafe { field_path.hash_unchecked() };
                                    field_values.get(&field_key).is_some()
                                })
                        };
                        if !has_field_values {
                            return;
                        }

                        ui.scope(|ui| {
                            let id = ui.id().with(field_key);
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                id,
                                false,
                            )
                            .show_header(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = emath::Vec2::ZERO;

                                    ui.label(field.var_name.to_string());
                                    ui.label(": ");

                                    ui.strong(field.var_type.to_string());
                                });
                            })
                            .body(|ui| {
                                let mut field_path = field_path.clone();
                                field_path.position += 1;
                                Self::show_fields(
                                    ui,
                                    &field_serializer.fields,
                                    field_values,
                                    field_path,
                                );
                            });
                        });
                    }
                }
            }
        });
    }
}
