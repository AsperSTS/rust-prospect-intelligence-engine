use super::*;
use prospect_core::{
    QueryGroup, QueryOrder,
    workspace::{BatchSummary, Density, TableColumn, TableLayout},
};

#[component]
pub(super) fn StateImportAction(
    entity: String,
    pending: usize,
    new: usize,
    configured: bool,
    running: bool,
) -> Element {
    let ui = use_context::<UiState>();
    let db = consume_context::<Services>().db;
    let label = if pending > 0 {
        "Reanudar"
    } else if new > 0 {
        "Importar"
    } else {
        "Consultado"
    };
    rsx! {button{class:"button secondary compact",disabled:running||!configured||(pending==0&&new==0),aria_label:format!("{label} datos de {}",prospect_core::config::entity_name(&entity)),onclick:move |_|job_start_scoped(if pending>0{JobKind::Resume}else{JobKind::Import},ui,db.clone(),String::new(),Some(entity.clone())),"{label}"}}
}

#[component]
pub(super) fn SavedWorkViews(mut query: Signal<Query>, mut layout: Signal<TableLayout>) -> Element {
    let mut ui = use_context::<UiState>();
    let db = consume_context::<Services>().db;
    let list_db = db.clone();
    let mut name = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let profile = use_memo(move || query.read().profile);
    let views = use_resource(move || {
        let _ = *ui.revision.read();
        let profile = *profile.read();
        let db = list_db.clone();
        async move { db.saved_views(profile).await.map_err(|e| e.to_string()) }
    });
    let state = views.read().clone();
    rsx! {
        details{class:"work-views",
            summary{"Vistas guardadas"}
            div{class:"work-views-body",
                p{class:"muted small","Guarda filtros, columnas, orden, agrupación y densidad para esta oferta. Se abre desde la primera página; el mismo nombre actualiza la vista."}
                div{class:"view-save-row",
                    TextEntry{label:"Nombre de la vista",initial:name.read().clone(),placeholder:"Ej. Proveedores del Bajío",oninput:move |e:FormEvent|name.set(e.value())}
                    button{class:"button secondary compact",disabled:*saving.read()||name.read().trim().is_empty(),onclick:move |_|{
                        saving.set(true); let db=db.clone();let name=name.read().clone();let query=query.read().clone();let layout=layout.read().clone();
                        spawn(async move {match db.save_view(name,query,layout).await {Ok(())=>{ui.revision+=1;ui.toast.set("Vista guardada para esta oferta".into());},Err(e)=>ui.toast.set(e.to_string())}saving.set(false);});
                    },if *saving.read(){"Guardando…"}else{"Guardar / actualizar"}}
                }
                match state {
                    None=>rsx!{p{class:"muted small",role:"status","Cargando vistas…"}},
                    Some(Err(error))=>rsx!{p{class:"error",role:"alert","{error}"}button{class:"link-button",onclick:move |_|{let mut views=views;views.restart();},"Reintentar"}},
                    Some(Ok(items))=>rsx!{
                        if items.is_empty(){p{class:"muted small","Aún no hay vistas para esta oferta."}}
                        div{class:"view-pills",for view in items {button{class:"pill-button",onclick:move |_|{let mut q=view.query.clone();q.page=0;q.view=View::All;query.set(q);layout.set(view.layout.clone());name.set(view.name.clone());},"{view.name}"}}}
                    },
                }
            }
        }
    }
}

#[component]
pub(super) fn TableControls(mut query: Signal<Query>, mut layout: Signal<TableLayout>) -> Element {
    let columns = layout.read().columns.clone();
    rsx! {details{class:"table-controls",
        summary{"Personalizar tabla"}
        div{class:"table-controls-body",
            div{class:"view-selectors",
                label{"Orden" select{value:QueryOrder::ALL.iter().position(|x|*x==query.read().order).unwrap_or(0).to_string(),onchange:move |e|{if let Some(order)=e.value().parse::<usize>().ok().and_then(|i|QueryOrder::ALL.get(i)){let mut q=query.write();q.order = *order;q.page=0;}},for (i,order) in QueryOrder::ALL.iter().enumerate(){option{value:i.to_string(),selected:*order==query.read().order,"{order.label()}"}}}}
                label{"Agrupar por" select{value:QueryGroup::ALL.iter().position(|x|*x==query.read().group).unwrap_or(0).to_string(),onchange:move |e|{if let Some(group)=e.value().parse::<usize>().ok().and_then(|i|QueryGroup::ALL.get(i)){let mut q=query.write();q.group = *group;q.page=0;}},for (i,group) in QueryGroup::ALL.iter().enumerate(){option{value:i.to_string(),selected:*group==query.read().group,"{group.label()}"}}}}
                label{"Densidad" select{value:if layout.read().density==Density::Compact{"compact"}else{"comfortable"},onchange:move |e|{layout.write().density=if e.value()=="compact"{Density::Compact}else{Density::Comfortable};},option{value:"comfortable",selected:layout.read().density==Density::Comfortable,"Cómoda"}option{value:"compact",selected:layout.read().density==Density::Compact,"Compacta"}}}
                label{"Filas por página" select{value:(if query.read().size==0{60}else{query.read().size}).to_string(),onchange:move |e|{if let Ok(size)=e.value().parse::<usize>() && [30,40,60,120,240,500].contains(&size){let mut q=query.write();q.size=size;q.page=0;}},for size in [30,40,60,120,240,500]{option{value:size.to_string(),selected:size==if query.read().size==0{60}else{query.read().size},"{size}"}}}}
            }
            div{class:"column-editor",role:"group",aria_label:"Columnas visibles y su orden",
                for (index,column) in columns.iter().copied().enumerate(){div{class:"column-option",
                    label{class:"check-label",input{r#type:"checkbox",checked:true,disabled:column==TableColumn::Name,onchange:move |_|layout.write().columns.retain(|c|*c!=column)}"{column.label()}"}
                    if column!=TableColumn::Name {
                        button{class:"column-move",aria_label:format!("Mover {} a la izquierda",column.label()),disabled:index<=1,onclick:move |_|{let mut l=layout.write();if index>1 && index<l.columns.len(){l.columns.swap(index,index-1);}},"←"}
                        button{class:"column-move",aria_label:format!("Mover {} a la derecha",column.label()),disabled:index+1>=columns.len(),onclick:move |_|{let mut l=layout.write();if index+1<l.columns.len(){l.columns.swap(index,index+1);}},"→"}
                    }
                }}
                for column in TableColumn::ALL.into_iter().filter(|c|!columns.contains(c)) {label{class:"column-option check-label",input{r#type:"checkbox",checked:false,onchange:move |_|{let mut l=layout.write();if !l.columns.contains(&column){l.columns.push(column);}}}"{column.label()}"}}
            }
            p{class:"muted small","El orden y los grupos se aplican a todos los resultados antes de paginar. Guarda una vista para conservar esta presentación."}
            button{class:"link-button",onclick:move |_|{layout.set(TableLayout::default());let mut q=query.write();q.order=QueryOrder::Automatic;q.group=QueryGroup::None;q.size=60;q.page=0;},"Restablecer presentación"}
        }
    }}
}

#[component]
pub(super) fn BatchHistory(profile: Profile) -> Element {
    let ui = use_context::<UiState>();
    let db = consume_context::<Services>().db;
    let batches = use_resource(move || {
        let _ = *ui.revision.read();
        let db = db.clone();
        async move {
            db.denue_batch_summaries(profile)
                .await
                .map_err(|e| e.to_string())
        }
    });
    let state = batches.read().clone();
    rsx! {section{class:"panel batch-history",
        div{class:"panel-heading",h2{"Cambios por lote · DENUE"}span{class:"muted small","Últimos 20 de esta oferta"}}
        div{class:"batch-history-body",
            match state {
                None=>rsx!{p{role:"status","Cargando lotes…"}},
                Some(Err(error))=>rsx!{p{class:"error",role:"alert","{error}"}button{class:"link-button",onclick:move |_|{let mut batches=batches;batches.restart();},"Reintentar"}},
                Some(Ok(items))=>rsx!{
                    if items.is_empty(){p{class:"muted","Los próximos lotes guardarán aquí su resumen. Los anteriores no tienen un detalle de campos reconstruible."}}
                    for batch in items {BatchCard{batch}}
                },
            }
            p{class:"muted small","Sólo páginas guardadas, incluso si el lote se canceló o falló después. Los conteos son registros procesados, no empresas distintas. Campos vacíos indica que DENUE dejó de aportar un valor; no se infieren cierres ni se modifica el historial comercial. La fecha corresponde a la consulta, no a la publicación."}
        }
    }}
}

#[component]
fn BatchCard(batch: BatchSummary) -> Element {
    let status = match batch.status.as_str() {
        "completed" => "Terminado",
        "cancelled" => "Cancelado",
        "failed" => "Con error",
        "running" => "En curso",
        _ => "Interrumpido",
    };
    rsx! {article{class:"batch-card",
        div{class:"batch-heading",strong{"{batch.kind}"}span{class:"badge","{status}"}span{class:"muted small","Consulta: {batch.queried_at}"}}
        dl{class:"batch-counts",for (label,value) in [("Páginas",batch.pages),("Nuevos",batch.inserted),("Modificados",batch.updated),("Sin cambios",batch.unchanged),("Inválidos",batch.invalid)]{div{dt{"{label}"}dd{"{number(value)}"}}}}
        if batch.changes.records_with_losses>0{p{class:"batch-loss",strong{if batch.changes.records_with_losses==1{"1 registro con campos que quedaron vacíos"}else{"{batch.changes.records_with_losses} registros con campos que quedaron vacíos"}}}}
        if !batch.changes.modified.is_empty(){details{summary{"Ver campos modificados"}
            div{class:"table-wrap",table{thead{tr{th{scope:"col","Campo público"}th{scope:"col","Cambios"}th{scope:"col","Quedaron vacíos"}}}tbody{for (field,n) in &batch.changes.modified{tr{td{"{prospect_core::workspace::field_label(field)}"}td{"{n}"}td{"{batch.changes.emptied.get(field).copied().unwrap_or(0)}"}}}}}}
        }}
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[component]
    fn RestoredControls() -> Element {
        let query = use_signal(|| Query {
            size: 60,
            order: QueryOrder::Name,
            group: QueryGroup::Municipality,
            ..Default::default()
        });
        let layout = use_signal(|| TableLayout {
            density: Density::Compact,
            ..Default::default()
        });
        rsx! {TableControls{query,layout}}
    }
    #[test]
    fn initial_controls_show_the_actual_restored_presentation() {
        let mut dom = VirtualDom::new(RestoredControls);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for label in ["Nombre · A–Z", "Municipio", "Compacta", "60"] {
            let option = html
                .split("<option")
                .filter_map(|p| p.split_once('>'))
                .find(|(_, body)| body.split("</option>").next() == Some(label))
                .unwrap();
            assert!(
                option.0.contains("selected"),
                "Restored value not selected: {label}"
            );
        }
    }
}
