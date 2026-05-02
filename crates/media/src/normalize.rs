use once_cell::sync::Lazy;
use opencc_jieba_rs::OpenCC;

static OPENCC: Lazy<OpenCC> = Lazy::new(OpenCC::new);

pub fn normalize_name(name: &str, for_search: bool) -> String {
    let mut s = OPENCC.t2s(name.trim(), false).to_lowercase();
    s = kana_to_hira(&s);
    if for_search {
        s.retain(|c| !c.is_whitespace());
    }
    s
}

fn kana_to_hira(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'ァ' => 'ぁ',
            'ィ' => 'ぃ',
            'ゥ' => 'ぅ',
            'ェ' => 'ぇ',
            'ォ' => 'ぉ',
            'ャ' => 'ゃ',
            'ュ' => 'ゅ',
            'ョ' => 'ょ',
            'ッ' => 'っ',
            'ア' => 'あ',
            'イ' => 'い',
            'ウ' => 'う',
            'エ' => 'え',
            'オ' => 'お',
            'カ' => 'か',
            'キ' => 'き',
            'ク' => 'く',
            'ケ' => 'け',
            'コ' => 'こ',
            'サ' => 'さ',
            'シ' => 'し',
            'ス' => 'す',
            'セ' => 'せ',
            'ソ' => 'そ',
            'タ' => 'た',
            'チ' => 'ち',
            'ツ' => 'つ',
            'テ' => 'て',
            'ト' => 'と',
            'ナ' => 'な',
            'ニ' => 'に',
            'ヌ' => 'ぬ',
            'ネ' => 'ね',
            'ノ' => 'の',
            'ハ' => 'は',
            'ヒ' => 'ひ',
            'フ' => 'ふ',
            'ヘ' => 'へ',
            'ホ' => 'ほ',
            'マ' => 'ま',
            'ミ' => 'み',
            'ム' => 'む',
            'メ' => 'め',
            'モ' => 'も',
            'ヤ' => 'や',
            'ユ' => 'ゆ',
            'ヨ' => 'よ',
            'ラ' => 'ら',
            'リ' => 'り',
            'ル' => 'る',
            'レ' => 'れ',
            'ロ' => 'ろ',
            'ワ' => 'わ',
            'ヲ' => 'を',
            'ン' => 'ん',
            'ヴ' => 'ゔ',
            'ヵ' => 'ゕ',
            'ヶ' => 'ゖ',
            _ => ch,
        })
        .collect()
}

pub fn fuzzy_score(attribute: &str, query: &str) -> f64 {
    let a = normalize_name(attribute, true);
    let q = normalize_name(query, true);
    if q.is_empty() {
        return 0.0;
    }
    if a == q {
        return 100.0;
    }
    if a.contains(&q) {
        return 88.0 + (q.len() as f64 / a.len().max(1) as f64) * 10.0;
    }
    let dist = levenshtein(&a, &q);
    let max_len = a.chars().count().max(q.chars().count()).max(1) as f64;
    ((1.0 - dist as f64 / max_len) * 100.0).max(0.0)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut costs: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut last = i;
        costs[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let old = costs[j + 1];
            costs[j + 1] = if ca == *cb {
                last
            } else {
                1 + last.min(costs[j]).min(old)
            };
            last = old;
        }
    }
    costs[b_chars.len()]
}
