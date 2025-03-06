use std::{
    collections::{BTreeMap, HashMap},
    io::{Cursor, Read},
    path::Path,
};

use anyhow::{Context, Result};
use git2::{build::RepoBuilder, Repository};
use once_cell::unsync::Lazy;
use pdb::{FallibleIterator, SymbolData, TypeData, PDB as Pdb};
use reqwest::{Client, Url};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWrite, AsyncWriteExt},
};
use zip::ZipArchive;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();

    println!("Fetching releases...");
    let mut releases = Vec::new();
    let mut url = Url::parse("https://api.github.com/repos/OpenRCT2/OpenRCT2/releases")
        .context("could not parse releases url")?;
    loop {
        let page = client
            .get(url.as_str())
            .header("User-Agent", "openrct2-autosplitter generator")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("could not fetch OpenRCT2 releases")?;

        let link = page
            .headers()
            .get("link")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| parse_link_header::parse(value).ok())
            .unwrap_or_default();

        let mut page: Vec<Release> = page
            .json()
            .await
            .context("could not parse releases as json")?;
        releases.append(&mut page);

        match link.get(&Some("next".to_string())) {
            Some(link) => url = Url::parse(&link.raw_uri).context("could not parse next url")?,
            None => break,
        }
    }

    let repo_info: Lazy<Result<RepoInfo>> = Lazy::new(|| {
        println!("Cloning repository...");
        checkout_repo()
    });

    println!("Creating output file...");
    let mut output = File::create("OpenRCT2_header.asl").await?;
    output
        .write_all(b"// OpenRCT2 Autosplitter by anna\n")
        .await?;
    output
        .write_all(b"// https://github.com/anna-is-cute/openrct2-autosplitter\n\n")
        .await?;

    println!("Getting cache...");
    let mut cache = Cache::get().await?;

    println!("Processing releases...");
    let mut hashes = Vec::with_capacity(releases.len());
    let mut offsets = Vec::with_capacity(releases.len());
    for release in releases {
        let res = process_release(
            &release,
            &mut cache,
            &repo_info,
            &mut hashes,
            &client,
            &mut offsets,
        ).await;
        if let Err(e) = res {
            eprintln!("could not process release {}: {}", release.name, e);
        }
    }

    println!("  Saving cache...");
    cache.save().await?;

    let mut supports_load_remover = HashMap::with_capacity(offsets.len());
    for (release_ref, has_dll, offsets) in offsets {
        output
            .write_all(format!("state(\"openrct2\", \"{}\") {{\n", release_ref).as_bytes())
            .await?;
        offsets.write_offsets(&mut output, has_dll).await?;
        output.write_all(b"}\n\n").await?;
        supports_load_remover.insert(release_ref, offsets.load_remover_will_work());
    }

    output.write_all(b"init {\n    var module = modules.First();\n    string hash = vars.CalcModuleHash(module);\n    switch (hash) {\n").await?;
    for (release_ref, hash) in hashes {
        let supports_load_remover = supports_load_remover
            .get(&release_ref)
            .copied()
            .unwrap_or_default();
        output
            .write_all(
                format!(
                    "        case \"{}\":\n            version = \"{}\";\n            vars.loadRemover = {};\n            break;\n",
                    hash,
                    release_ref,
                    if supports_load_remover {
                        "true"
                    } else {
                        "false"
                    }
                )
                .as_bytes(),
            )
            .await?;
    }
    // double newline for concatenation purposes
    output.write_all(b"    }\n}\n\n").await?;

    Ok(())
}

async fn process_release(
    release: &Release,
    cache: &mut Cache,
    repo_info: &Lazy<Result<RepoInfo>>,
    hashes: &mut Vec<(String, String)>,
    client: &Client,
    offsets: &mut Vec<(String, bool, Offsets)>,
) -> Result<()> {
    let symbols_asset = match release
        .assets
        .iter()
        .find(|asset| asset.name.contains("symbols-x64"))
    {
        Some(asset) => asset,
        None => Err(anyhow::anyhow!(
            "Release {} is missing debug symbols",
            release.name
        ))?,
    };

    let binary_asset = match release
        .assets
        .iter()
        .find(|asset| asset.name.contains("windows-portable-x64"))
    {
        Some(asset) => asset,
        None => Err(anyhow::anyhow!(
            "Release {} is missing binaries",
            release.name
        ))?,
    };

    let release_ref = match cache.release_refs.get(&release.id) {
        Some(cached) => cached.clone(),
        None => {
            let info = match &**repo_info {
                Ok(info) => info,
                Err(e) => Err(anyhow::anyhow!("could not clone repository: {}", e))?,
            };
            let reference = info
                .repo
                .find_reference(&format!("refs/tags/{}", release.tag_name))?;
            let commit = reference.peel_to_commit()?;
            let sha = commit.id().to_string();
            format!("{} ({})", release.tag_name, &sha[..7])
        }
    };

    println!("  Downloading binaries for {}", release_ref);
    let (hash, has_dll) = match cache
        .assets
        .get(&(binary_asset.id, binary_asset.size))
        .and_then(|cached| cached.hash.as_ref().map(|hash| (hash, cached.has_dll)))
    {
        Some((hash, has_dll)) => (hash.clone(), has_dll),
        None => process_binary(client, binary_asset).await?,
    };
    hashes.push((release_ref.clone(), hash.clone()));

    println!("  Downloading symbols for {}", release_ref);
    let release_offsets = match cache
        .assets
        .get(&(symbols_asset.id, symbols_asset.size))
        .and_then(|cached| cached.offsets.as_ref())
    {
        Some(cached) => {
            let mut cached = cached.clone();
            if !cached.autosplitter_will_work() {
                process_symbols(client, symbols_asset, &mut cached).await?;
            }

            cached
        }
        _ => {
            let mut offsets = Offsets::default();
            process_symbols(client, symbols_asset, &mut offsets).await?;
            offsets
        }
    };

    if !release_offsets.autosplitter_will_work() {
        println!("    Warning: offsets state not valid - parts of the autosplitter may not work for this version");
    }

    if !release_offsets.load_remover_will_work() {
        println!("    Warning: missing loading information - load remover will not work for this version");
    }

    offsets.push((release_ref.clone(), has_dll, release_offsets.clone()));

    cache.release_refs.insert(release.id, release_ref);
    cache.assets.insert(
        (binary_asset.id, binary_asset.size),
        CacheAsset {
            id: binary_asset.id,
            size: binary_asset.size,
            hash: Some(hash),
            offsets: None,
            has_dll,
        },
    );
    cache.assets.insert(
        (symbols_asset.id, symbols_asset.size),
        CacheAsset {
            id: symbols_asset.id,
            size: symbols_asset.size,
            hash: None,
            offsets: Some(release_offsets),
            has_dll,
        },
    );

    Ok(())
}

async fn get_asset_zip(client: &Client, asset: &Asset) -> Result<ZipArchive<Cursor<Vec<u8>>>> {
    let zip_bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .bytes()
        .await?;
    let zip = ZipArchive::new(Cursor::new(zip_bytes.to_vec()))?;
    Ok(zip)
}

async fn process_binary(client: &Client, asset: &Asset) -> Result<(String, bool)> {
    let mut zip = get_asset_zip(client, asset).await?;
    let has_dll = zip.file_names().any(|name| name == "openrct2.dll");

    let mut zip_exe = zip.by_name("openrct2.exe")?;
    let mut raw_exe = vec![0; zip_exe.size() as usize];
    zip_exe.read_exact(&mut raw_exe)?;

    Ok((hex::encode(Sha256::digest(&*raw_exe)), has_dll))
}

async fn process_symbols(client: &Client, asset: &Asset, offsets: &mut Offsets) -> Result<()> {
    let zip_bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .bytes()
        .await?;
    let mut zip = ZipArchive::new(Cursor::new(&*zip_bytes))?;
    let names: Vec<String> = zip.file_names().map(ToString::to_string).collect();
    for name in names {
        if name.ends_with(".pdb") {
            let mut zip_pdb = zip.by_name(&name)?;
            let mut raw_pdb = vec![0; zip_pdb.size() as usize];
            zip_pdb.read_exact(&mut raw_pdb)?;
            let mut pdb = Pdb::open(Cursor::new(&*raw_pdb))?;
            let addr_map = pdb.address_map()?;

            // check for the GameState struct
            let types = pdb.type_information()?;
            let mut iter = types.iter();
            let mut parent_fields_idx = Vec::new();
            while let Ok(Some(type_)) = iter.next() {
                if let Ok(TypeData::Class(class)) = type_.parse() {
                    if class.name.to_string() != "OpenRCT2::GameState_t" {
                        continue;
                    }

                    let fields_idx = match class.fields {
                        Some(fields) => fields,
                        None => continue,
                    };

                    parent_fields_idx.push(fields_idx);
                }
            }

            // scan again for the GameState field list and find
            // CompletedScenarioValue
            let mut iter = types.iter();
            while let Ok(Some(type_)) = iter.next() {
                if parent_fields_idx.iter().all(|&idx| idx != type_.index()) {
                    continue;
                }

                if let Ok(TypeData::FieldList(list)) = type_.parse() {
                    let field = list
                        .fields
                        .iter()
                        .flat_map(|field| match field {
                            TypeData::Member(member) => Some(member),
                            _ => None,
                        })
                        .find(|member| member.name.to_string() == "ScenarioCompletedCompanyValue");
                    if let Some(field) = field {
                        offsets.game_state_completed_value = Some(field.offset);
                    }
                }
            }

            // look for the globals
            let globals = pdb.global_symbols()?;
            let mut iter = globals.iter();
            while let Ok(Some(symbol)) = iter.next() {
                if let Ok(SymbolData::Data(d)) = symbol.parse() {
                    let field = match d.name.to_string().as_ref() {
                        "gScreenFlags" => &mut offsets.screen_flags,
                        "gScenarioCompletedCompanyValue" => &mut offsets.completed_value,
                        "OpenRCT2::_gameState" => {
                            offsets.game_state_is_pointer = Some(true);

                            &mut offsets.game_state
                        }
                        "_gameState" => {
                            offsets.game_state_is_pointer = Some(false);

                            &mut offsets.game_state
                        }
                        "_mapChangedExpected" => &mut offsets.map_changed_expected,
                        _ => continue,
                    };

                    if field.is_some() {
                        continue;
                    }

                    if let Some(offset) = d.offset.to_rva(&addr_map) {
                        *field = Some(offset.0);
                    }
                }
            }
        }
    }

    Ok(())
}

fn checkout_repo() -> Result<RepoInfo> {
    let path = tempfile::tempdir()?;
    let repo = RepoBuilder::new()
        .bare(true)
        .clone("https://github.com/OpenRCT2/OpenRCT2", path.as_ref())?;
    Ok(RepoInfo { repo, _path: path })
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Release {
    id: u64,
    name: String,
    assets: Vec<Asset>,
    tag_name: String,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Asset {
    id: u64,
    size: u64,
    browser_download_url: String,
    name: String,
}

#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Cache {
    release_refs: BTreeMap<u64, String>,
    assets: BTreeMap<(u64, u64), CacheAsset>,
}

impl Cache {
    const FILE_NAME: &'static str = "cache.ron";

    async fn get() -> Result<Self> {
        let path = Path::new(Self::FILE_NAME);
        if path.exists() {
            let file = File::open(Self::FILE_NAME).await?;
            let cache: Self = ron::de::from_reader(file.into_std().await)?;
            Ok(cache)
        } else {
            File::create(path).await?;
            Ok(Self::default())
        }
    }

    async fn save(&self) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(Self::FILE_NAME)
            .await?;
        ron::ser::to_writer_pretty(file.into_std().await, self, PrettyConfig::default())?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct CacheAsset {
    id: u64,
    size: u64,
    offsets: Option<Offsets>,
    hash: Option<String>,
    has_dll: bool,
}

#[derive(Default, Deserialize, Serialize, Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Offsets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    screen_flags: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_value: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    game_state: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    game_state_is_pointer: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    game_state_completed_value: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    map_changed_expected: Option<u32>,
}

impl Offsets {
    pub fn autosplitter_will_work(&self) -> bool {
        let has_screen_flags = self.screen_flags.is_some();
        let has_completed_value = self.completed_value.is_some();
        let has_game_state_data =
            self.game_state.is_some() && self.game_state_completed_value.is_some();
        let has_pointer_info = self.game_state_is_pointer.is_some();
        has_screen_flags && (has_completed_value ^ (has_game_state_data && has_pointer_info))
    }

    pub fn load_remover_will_work(&self) -> bool {
        self.map_changed_expected.is_some()
    }

    async fn write_offset<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
        has_dll: bool,
        name: &str,
        kind: &str,
        offsets: &[usize],
    ) -> Result<()> {
        let mut offsets = offsets
            .iter()
            .map(|offset| format!("0x{offset:x}"))
            .collect::<Vec<_>>();
        if has_dll {
            offsets.insert(0, "\"openrct2.dll\"".into());
        }
        let offsets = offsets.join(", ");

        writer
            .write_all(format!("    {kind} {name} : {offsets};\n").as_bytes())
            .await?;

        Ok(())
    }

    pub async fn write_offsets<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
        has_dll: bool,
    ) -> Result<()> {
        if let Some(offset) = self.screen_flags {
            self.write_offset(writer, has_dll, "gScreenFlags", "byte", &[offset as usize])
                .await?;
        }

        if let Some(offset) = self.completed_value {
            self.write_offset(
                writer,
                has_dll,
                "gScenarioCompletedCompanyValue",
                "ulong",
                &[offset as usize],
            )
            .await?;
        }

        if let (Some(state_offset), Some(field_offset), Some(is_pointer)) = (
            self.game_state,
            self.game_state_completed_value,
            self.game_state_is_pointer,
        ) {
            let offsets = if is_pointer {
                vec![state_offset as usize, field_offset as usize]
            } else {
                vec![state_offset as usize + field_offset as usize]
            };

            self.write_offset(
                writer,
                has_dll,
                "gScenarioCompletedCompanyValue",
                "ulong",
                &offsets,
            )
            .await?;
        }

        if let Some(offset) = self.map_changed_expected {
            self.write_offset(
                writer,
                has_dll,
                "_mapChangedExpected",
                "byte",
                &[offset as usize],
            )
            .await?;
        }

        Ok(())
    }
}

struct RepoInfo {
    repo: Repository,
    _path: TempDir,
}
