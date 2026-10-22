#![forbid(unsafe_code)]
use clap::{Parser, Subcommand};
use prospect_core::{AppPaths, Db, Profile, Query, Settings, View, jobs::JobControl};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "prospectos",
    version,
    about = "Prospect Intelligence · Rust local"
)]
struct Args {
    #[arg(
        long,
        global = true,
        help = "Directorio local de datos; por defecto Application Support"
    )]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Clone, Copy, clap::ValueEnum)]
enum PullMode {
    Nuevos,
    Reanudar,
    Revisar,
}
impl PullMode {
    fn core(self) -> prospect_core::updates::ImportMode {
        match self {
            Self::Nuevos => prospect_core::updates::ImportMode::New,
            Self::Reanudar => prospect_core::updates::ImportMode::Resume,
            Self::Revisar => prospect_core::updates::ImportMode::Recheck,
        }
    }
}
#[derive(Subcommand)]
enum Commands {
    /// Copia consistente de SQLite legacy; nunca sobrescribe una base Rust existente.
    Migrate {
        source: PathBuf,
    },
    /// Conteos e integridad; no imprime contactos ni secretos.
    Inspect,
    Daily,
    Analizar,
    Pull {
        #[arg(long, default_value = "automatizacion")]
        profile: String,
        #[arg(long, value_delimiter = ',')]
        entidades: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        estratos: Vec<String>,
        #[arg(
            long,
            help = "Presupuesto global: múltiplos de 200, entre 200 y 4000; sin opción usa Ajustes"
        )]
        limite: Option<usize>,
        #[arg(long, value_enum, default_value = "nuevos")]
        modo: PullMode,
    },
    Cuantificar {
        #[arg(long, default_value = "21")]
        entidad: String,
        #[arg(long, default_value = "31")]
        sector: String,
        #[arg(long, default_value = "3")]
        estrato: String,
    },
    Enriquecer {
        #[arg(long, default_value_t = 100)]
        dominios: usize,
        #[arg(long)]
        forzar: bool,
    },
    Run {
        #[arg(long, default_value = "automatizacion")]
        profile: String,
        #[arg(long, value_delimiter = ',')]
        entidades: Vec<String>,
        #[arg(
            long,
            help = "Presupuesto global: múltiplos de 200, entre 200 y 4000; sin opción usa Ajustes"
        )]
        limite: Option<usize>,
        #[arg(long, value_enum, default_value = "nuevos")]
        modo: PullMode,
        #[arg(long, default_value_t = 100)]
        enriquecer: usize,
    },
    Export {
        #[arg(long,default_value="xlsx",value_parser=["xlsx","csv"])]
        format: String,
        #[arg(long, default_value = "automatizacion")]
        profile: String,
        #[arg(long, default_value = "")]
        search: String,
    },
    Backup,
    Recover {
        destination: PathBuf,
    },
    Indicators {
        #[arg(value_parser=["banxico","inegi"])]
        source: String,
        #[arg(long, default_value = "1002000001")]
        indicator: String,
    },
    /// Ejecuta una acción CRM tipada, descrita en un archivo JSON local.
    Mutate {
        file: PathBuf,
    },
    List {
        #[arg(long, default_value = "")]
        search: String,
        #[arg(long, default_value = "automatizacion")]
        profile: String,
        #[arg(long, default_value_t = 20)]
        size: usize,
    },
    /// Exporta análisis para comparación de migración, a un archivo local.
    Parity {
        output: PathBuf,
        #[arg(long)]
        date: Option<String>,
    },
}
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn run() -> prospect_core::Result<()> {
    let args = Args::parse();
    let paths = if let Some(home) = args.home {
        AppPaths::at(home)?
    } else {
        AppPaths::discover()?
    };
    if let Commands::Migrate { source } = &args.command {
        let result = prospect_core::db::migrate_snapshot(source, &paths)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    let db = Db::open(paths.clone())?;
    let mut settings = Settings::load(&paths)?;
    let control = JobControl::new("cli");
    let progress = control.clone();
    let monitor = tokio::spawn(async move {
        let mut last = String::new();
        loop {
            let state = progress.snapshot();
            if state.message != last {
                eprintln!("{}", state.message);
                last = state.message;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    match args.command {
        Commands::Inspect => {
            let result=db.call(|c|{prospect_core::db::verify(c)?;Ok(serde_json::json!({"integrity":"ok","legacy_version":c.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0))?,"sqlite_version":rusqlite_version(c)?,"tables":prospect_core::db::table_counts(c)?}))}).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Daily => println!("{}", serde_json::to_string_pretty(&db.summary().await?)?),
        Commands::Analizar => {
            println!(
                "Analizados: {}",
                db.analyze(settings, control.clone()).await?
            );
        }
        Commands::Cuantificar {
            entidad,
            sector,
            estrato,
        } => {
            let secret = paths.secrets()?;
            let n = prospect_core::sources::GovernmentClient::new()?
                .quantify(&secret.denue, &entidad, &sector, &estrato, &control)
                .await?;
            println!("Establecimientos: {n}");
        }
        Commands::Pull {
            profile,
            entidades,
            estratos,
            limite,
            modo,
        } => {
            if !entidades.is_empty() {
                settings.entities = entidades;
            }
            if !estratos.is_empty() {
                settings.estratos = estratos;
            }
            settings.profile = profile.clone();
            settings.denue_pages =
                prospect_core::updates::record_limit_pages(limite, settings.denue_pages)?;
            let n = prospect_core::updates::pull_batch(
                &db,
                &settings,
                Profile::parse(&profile)?,
                modo.core(),
                &control,
            )
            .await?;
            db.analyze(settings, control.clone()).await?;
            println!("Registros procesados: {n}");
        }
        Commands::Enriquecer { dominios, forzar } => {
            let n = prospect_core::enrich::pending(&db, dominios, forzar, &control).await?;
            db.analyze(settings, control.clone()).await?;
            println!("Dominios: {n}");
        }
        Commands::Run {
            profile,
            entidades,
            limite,
            modo,
            enriquecer,
        } => {
            if !entidades.is_empty() {
                settings.entities = entidades;
            }
            settings.profile = profile.clone();
            settings.denue_pages =
                prospect_core::updates::record_limit_pages(limite, settings.denue_pages)?;
            let n = prospect_core::updates::pull_batch(
                &db,
                &settings,
                Profile::parse(&profile)?,
                modo.core(),
                &control,
            )
            .await?;
            db.analyze(settings.clone(), control.clone()).await?;
            if enriquecer > 0 {
                prospect_core::enrich::pending(&db, enriquecer, false, &control).await?;
                db.analyze(settings, control.clone()).await?;
            }
            println!("Registros procesados: {n}");
        }
        Commands::Export {
            format,
            profile,
            search,
        } => {
            let out = db
                .export(
                    Query {
                        view: View::All,
                        search,
                        profile: Profile::parse(&profile)?,
                        ..Default::default()
                    },
                    &format,
                )
                .await?;
            println!("{}", out.display());
        }
        Commands::Backup => println!("{}", db.backup().await?.display()),
        Commands::Recover { destination } => {
            println!("{}", db.restore_to(destination).await?.display())
        }
        Commands::Indicators { source, indicator } => {
            let n =
                prospect_core::sources::sync_economics(&db, &paths, &source, &indicator, &control)
                    .await?;
            println!("Observaciones guardadas: {n}");
        }
        Commands::Mutate { file } => {
            let m = serde_json::from_slice(&std::fs::read(file)?)?;
            println!("{}", db.mutate(m, settings).await?);
        }
        Commands::List {
            search,
            profile,
            size,
        } => {
            let page = db
                .page(Query {
                    view: View::All,
                    search,
                    profile: Profile::parse(&profile)?,
                    size,
                    ..Default::default()
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&page)?);
        }
        Commands::Parity { output, date } => {
            let date = if let Some(date) = date {
                prospect_core::config::parse_date(&date)?
            } else {
                prospect_core::config::today()
            };
            let templates = prospect_core::messages::Templates::load(&paths.home)?;
            let rows = db.call(prospect_core::db::all_mut).await?;
            let groups = prospect_core::groups::group(&rows);
            let contacted = rows
                .iter()
                .filter(|p| p.etapa_max >= 1 && groups[&p.id].possible_chain != 0)
                .map(|p| groups[&p.id].group_key.clone())
                .collect::<std::collections::HashSet<_>>();
            let mut result = Vec::new();
            for p in rows {
                let signals = db.signals(p.id.clone()).await?;
                let a = prospect_core::rules::analyze(
                    &p,
                    &signals,
                    groups[&p.id].clone(),
                    contacted.contains(&groups[&p.id].group_key),
                    date,
                    Profile::Automatizacion,
                );
                let mut rendered = p.clone();
                rendered.giro = a.giro.clone();
                rendered.giro_display = a.giro_display.clone();
                let wa = prospect_core::messages::first(
                    &rendered,
                    "whatsapp",
                    &signals,
                    &settings,
                    &templates,
                    Profile::Automatizacion,
                )?;
                let email = prospect_core::messages::first(
                    &rendered,
                    "email",
                    &signals,
                    &settings,
                    &templates,
                    Profile::Automatizacion,
                )?;
                result
                    .push(serde_json::json!({"id":p.id,"analysis":a,"whatsapp":wa,"email":email}));
            }
            prospect_core::config::write_private(&output, &serde_json::to_vec(&result)?)?;
            println!("Comparación local exportada: {} registros", result.len());
        }
        Commands::Migrate { .. } => {}
    }
    monitor.abort();
    Ok(())
}
fn rusqlite_version(c: &mut prospect_core::db::SqliteConnection) -> prospect_core::Result<String> {
    Ok(c.query_row("SELECT sqlite_version()", [], |r| r.get(0))?)
}
