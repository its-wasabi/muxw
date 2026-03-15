pub mod config;
pub use config::Config;
pub use config::XkbLayout;
pub use config::XkbOptions;
pub mod event;
pub use event::Device;
pub use event::Key;

fn table_to_comma_string(table: &mlua::Table) -> mlua::Result<String> {
    let items: Vec<String> = table
        .sequence_values()
        .collect::<Result<Vec<String>, _>>()?;
    Ok(items.join(","))
}
