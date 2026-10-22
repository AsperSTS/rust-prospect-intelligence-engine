use crate::{Group, Prospect};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;
pub fn normalize(s: &str) -> String {
    let ascii = s
        .nfkd()
        .filter(char::is_ascii)
        .collect::<String>()
        .to_lowercase()
        .replace('.', "");
    static PUNCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9 ]").unwrap());
    let ascii = PUNCT.replace_all(&ascii, " ");
    let mut s = ascii.split_whitespace().collect::<Vec<_>>().join(" ");
    static BRANCH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(sucursal|suc)\b.*$").unwrap());
    s = BRANCH.replace(&s, "").trim().into();
    static LEGAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(sapi de cv|sapi|sa de cv|sa|s de rl de cv|s de rl|sc de rl|sc|ac|sas)$")
            .unwrap()
    });
    for _ in 0..3 {
        s = LEGAL.replace(&s, "").trim().into();
    }
    s
}
fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}
pub fn group(rows: &[Prospect]) -> HashMap<String, Group> {
    let mut parents = (0..rows.len()).collect::<Vec<_>>();
    let keys = rows
        .iter()
        .map(|p| {
            let rs = normalize(&p.razon_social);
            [
                if rs.len() >= 5
                    && ![
                        "",
                        "sin razon social",
                        "no disponible",
                        "ninguna",
                        "ninguno",
                        "na",
                        "n a",
                    ]
                    .contains(&rs.as_str())
                {
                    rs
                } else {
                    String::new()
                },
                crate::contacts::domain(&p.sitio),
                crate::contacts::phone(&p.telefono),
            ]
        })
        .collect::<Vec<_>>();
    for kind in [0, 1, 2] {
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            if !k[kind].is_empty() {
                groups.entry(&k[kind]).or_default().push(i);
            }
        }
        for indices in groups.values() {
            for &i in &indices[1..] {
                let r = root(&mut parents, i);
                parents[r] = root(&mut parents, indices[0]);
            }
        }
    }
    let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..rows.len() {
        components.entry(root(&mut parents, i)).or_default().push(i);
    }
    let names = rows
        .iter()
        .map(|p| format!("{}|{}", normalize(&p.nombre), p.entidad))
        .collect::<Vec<_>>();
    let mut counts = HashMap::new();
    for n in &names {
        *counts.entry(n).or_insert(0i64) += 1;
    }
    let mut out = HashMap::new();
    for members in components.values() {
        if members.len() >= 2 {
            let mut links = Vec::new();
            for kind in [0, 1, 2] {
                let mut counts = HashMap::new();
                for &i in members {
                    let k = &keys[i][kind];
                    if !k.is_empty() {
                        *counts.entry(k).or_insert(0usize) += 1;
                    }
                }
                let shared = members
                    .iter()
                    .map(|&i| &keys[i][kind])
                    .find(|k| !k.is_empty() && counts.get(k).copied().unwrap_or(0) >= 2);
                if let Some(k) = shared {
                    links.push((kind, k.clone()));
                }
            }
            let labels = ["misma razón social", "mismo dominio web", "mismo teléfono"];
            let kinds = ["rs", "web", "tel"];
            for &i in members {
                out.insert(
                    rows[i].id.clone(),
                    Group {
                        group_key: format!("{}:{}", kinds[links[0].0], links[0].1),
                        possible_chain: 1,
                        possible_chain_count: members.len() as i64,
                        group_reason: links
                            .iter()
                            .map(|(k, _)| labels[*k])
                            .collect::<Vec<_>>()
                            .join(" + "),
                    },
                );
            }
        } else {
            let i = members[0];
            let n = counts[&names[i]];
            let weak = n >= 3 && !names[i].starts_with('|');
            out.insert(
                rows[i].id.clone(),
                Group {
                    group_key: if weak {
                        format!("nom:{}", names[i])
                    } else {
                        format!("id:{}", rows[i].id)
                    },
                    possible_chain: i64::from(weak),
                    possible_chain_count: if weak { n } else { 1 },
                    group_reason: if weak {
                        format!("mismo nombre en {n} establecimientos del estado (indicio débil)")
                    } else {
                        String::new()
                    },
                },
            );
        }
    }
    out
}
