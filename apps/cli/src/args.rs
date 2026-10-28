use std::path::PathBuf;

use clap::Parser;

/// 按优先级挑一个存在的数据文件：`data/generated/` 里打包好的 `.qj`、那里的 TSV、仓库自带的产品数据
/// （`assets/lexicon/dict.tsv`、`assets/glossary/glossary-*.tsv`、`assets/lexicon/english.tsv`），最后是 `assets/sample/` 的样例。
pub fn default_data_file(name: &str) -> PathBuf {
    let generated = PathBuf::from("data/generated").join(name);
    let packed = generated.with_extension("qj");
    let shipped = if name.starts_with("glossary-") {
        PathBuf::from("assets/glossary").join(name)
    } else {
        PathBuf::from("assets/lexicon").join(name)
    };
    for candidate in [packed, generated, shipped] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("assets/sample").join(name)
}

/// 缺省配置文件位置：与输入法共用同一份。
pub fn default_config_file() -> PathBuf {
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join("Library/Application Support/Qingjian/config.toml");
    }
    PathBuf::from("config.toml")
}

#[derive(Debug, Parser)]
#[command(name = "qingjian", about = "青简输入法 Core 测试工具")]
pub struct Args {
    /// 每次查询的 emoji 候选上限，0 关闭；缺省保持最多三个。
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(0..=3))]
    pub emoji_limit: u8,

    /// 词库路径（TSV）。缺省：data/generated/dict.tsv 存在就用它，否则 assets/sample/dict.tsv
    #[arg(long)]
    pub dict: Option<PathBuf>,

    /// 释义表路径。缺省：`data/generated/glossary-<language>.tsv` 存在就用它，否则 assets/sample/ 下的同名文件
    #[arg(long)]
    pub glossary: Option<PathBuf>,

    /// 学习语言：en / ja / es / vi / de。也可用环境变量 QINGJIAN_LEARNING_LANGUAGE
    #[arg(long, env = "QINGJIAN_LEARNING_LANGUAGE", default_value = "en")]
    pub language: String,

    /// 附加词库（.qj 或 TSV），可给多个，与主词库一起查
    #[arg(long)]
    pub extra_dict: Vec<PathBuf>,

    /// 通用词库（补充词库，排序低主词库一档）。缺省：data/generated/dicts/general.qj 存在就用它；给 `none` 不加载
    #[arg(long)]
    pub general_dict: Option<PathBuf>,

    /// 辅码码表（.qj，或 `词<TAB>码` 的 TSV；也可以是写了 `type = "aux"` 的方案目录，带上它的词码规则），可给多个一起筛。给了之后 `kaifa;kf` 这样的输入
    /// 按辅码态走：触发键进辅码态、之后的字母按码缩小候选
    #[arg(long)]
    pub aux_table: Vec<PathBuf>,

    /// 查码：打印这些词在已装码表里的全部码（配 --aux-table 用），逗号分隔或多次给；查完即退出
    #[arg(long, value_delimiter = ',')]
    pub aux_query: Vec<String>,

    /// 英文词表路径（中英混输）。缺省：data/generated/english.tsv 存在就用它，否则不启用
    #[arg(long)]
    pub english: Option<PathBuf>,

    /// 用户词频文件；给了就在退出时写回，不给则只在本次会话内学习
    #[arg(long)]
    pub user_dict: Option<PathBuf>,

    /// 配置文件路径。缺省：~/Library/Application Support/Qingjian/config.toml（macOS）或 ./config.toml
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// 启用云联想（无视配置里的 enabled）；密钥来自配置或 QINGJIAN_API_KEY（`api_key_env`）
    #[arg(long)]
    pub predict: bool,

    /// 模糊音，逗号分隔（z-zh,c-ch,s-sh,n-l,f-h,l-r,an-ang,en-eng,in-ing），`all` 全开；给了就覆盖配置里的 `[fuzzy]`
    #[arg(long, value_delimiter = ',')]
    pub fuzzy: Vec<String>,

    /// 英文模式（输入法里是 Caps Lock 亮着）：字母不当拼音，候选来自英文词表的补全与拼错纠正
    #[arg(long)]
    pub english_mode: bool,

    /// 关掉「打拼音时也给英文词」（配置 `[general]` english_in_chinese = false）：英文词既不进候选，
    /// 也不作为句尾参与整句分词
    #[arg(long)]
    pub no_english_in_chinese: bool,

    /// 关掉「长拼音首页固定单字」（配置 `[general]` first_page_characters = false）
    #[arg(long)]
    pub no_first_page_characters: bool,

    /// 打开中文优先（配置 `[general] chinese_first = true`）：整段是英文词时中文候选排第一、英文第二，评测两种排法用
    #[arg(long)]
    pub chinese_first: bool,

    /// 中文模式下英文候选（精确词 + 补全）最多几个（配置 `[general] english_max_in_chinese`，缺省 4）
    #[arg(long, default_value_t = qingjian_core::ENGLISH_CANDIDATES_MAX)]
    pub english_max_in_chinese: usize,

    /// 有中文候选时英文最高排到第几位（配置 `[general] english_min_position`，缺省 1 即不限制）
    #[arg(long, default_value_t = 1)]
    pub english_min_position: usize,

    /// 候选旁显示带调拼音（配置 `[general]` show_tones = true）
    #[arg(long)]
    pub show_tones: bool,

    /// 双拼方案（xiaohe / ziranma / microsoft / sogou / abc / xiaolang / shoudao / jiajia / ziguang，或用户双拼方案的目录），覆盖配置里的 `[general]` scheme；off 强制全拼
    #[arg(long)]
    pub shuangpin: Option<String>,

    /// 九宫格方案：输入写数字键（`64426` → 你好），验证手机 9 键的候选与整句
    #[arg(long)]
    pub nine_key: bool,

    /// 形码：码表 TSV（`词\t编码\t词频`，按 86 五笔记日志、按键规则全关）或方案目录（`scheme.toml` + 码表）。
    /// 给了就开着形码；`--wubi` 是旧写法
    #[arg(long, alias = "wubi", value_name = "码表或方案目录")]
    pub code: Option<PathBuf>,

    /// 神经重打分：字级 Transformer 的 .qjm 文件或导出目录（model.safetensors / config.json / vocab.json），整句前几条路径用它重排
    #[arg(long)]
    pub neural: Option<PathBuf>,

    /// 神经重打分的权重 λ（0 到 1，缺省 0.5）：最终分 = 路径分 + λ·(神经分 − 静态二元分)，个人学习与代价不受影响
    #[arg(long)]
    pub neural_weight: Option<f64>,

    /// 神经重打分的门槛（nat，缺省不设）：路径分落后最优路径超过这么多的不参与重排
    #[arg(long)]
    pub neural_margin: Option<f64>,

    /// 神经重打分给模型看的前文字符数（缺省 64，0 为不给前文）
    #[arg(long)]
    pub neural_context: Option<usize>,

    /// 试验：回放时用知意的选词模型重排第一页候选（解压的 safetensors 模型目录）
    #[arg(long)]
    pub zhiyi: Option<PathBuf>,

    /// 知意重排看前这么多个候选（知意是 2 × 每页候选数）
    #[arg(long, default_value_t = 18)]
    pub zhiyi_top: usize,

    /// 知意重排里语言模型分的权重（知意配它自己的三元模型是 1.0）
    #[arg(long, default_value_t = 1.0)]
    pub zhiyi_lm_weight: f64,

    /// 知意重排里中文位置先验的权重（我们原来的顺序带着个人学习；知意是 0）
    #[arg(long, default_value_t = 0.0)]
    pub zhiyi_rank_weight: f64,

    /// 神经重打分走后台线程（输入法壳里的接法）：查询先按词级模型出候选，再请求 / 等待重打分后重查一次；结果应与同步一致
    #[arg(long)]
    pub neural_async: bool,

    /// 逐键模式：把每个输入当作一键一键敲进去，每个前缀都查一次，打印每键各阶段耗时（性能测试用）
    #[arg(long)]
    pub typing: bool,

    /// 只显示前 N 个候选
    #[arg(long, default_value_t = 9)]
    pub limit: usize,

    /// 回放评测：读输入日志（input-log.jsonl），把每次上屏时的键重新喂给引擎，算首选命中率等指标。只在内存里学习，不写任何文件
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// 从输入日志挖词库缺词：上屏文字按语言模型切词，连续单字段与「模型认识、词库没有」的词
    /// 按出现次数列成 `词\t拼音\t次数` 清单，人工过目后并进词库；只读日志，不写任何文件
    #[arg(long, conflicts_with_all = ["replay", "eval_text", "eval_cold"])]
    pub mine_log: Option<PathBuf>,

    /// 挖词的最低出现次数（配 --mine-log，缺省 2）
    #[arg(long, default_value_t = 2, requires = "mine_log")]
    pub mine_min_count: u32,

    /// 挖词的最长字数（配 --mine-log，缺省 6）
    #[arg(long, default_value_t = 6, requires = "mine_log")]
    pub mine_max_chars: usize,

    /// 把过阈值的**全拼**行直接并进用户词（配 --mine-log）：给了 --user-dict 写回同目录的
    /// user-words.tsv，不给则只打印预演；简拼行单独列出等人工补读音
    #[arg(long, requires = "mine_log")]
    pub mine_apply: bool,

    /// 按读音核对用户词（配 --user-dict）：逐条列出读音与主词库对不上的用户词，只读、不写文件
    #[arg(long, conflicts_with_all = ["mine_log", "replay", "eval_text", "eval_cold"])]
    pub check_user_words: bool,

    /// 把核对不过的用户词删掉并落盘（配 --check-user-words，落盘目标由 --user-dict 指定）；
    /// 删完读回 user-words.tsv 核实，没删干净就报错退出
    #[arg(long, requires = "check_user_words")]
    pub purge_user_words: bool,

    /// 导入无读音词表（每行 `词`、`词<Tab>词频` 或 `词<Tab>读音[<Tab>词频]`）：读音按 --dict、--extra-dict 与随包领域词库取，一个词一条；
    /// 只读、不写文件，配 --import-to 才落盘
    #[arg(long, value_name = "词表", conflicts_with_all = ["mine_log", "replay", "eval_text", "eval_cold", "check_user_words", "aux_query"])]
    pub import_words: Option<PathBuf>,

    /// 落盘目录（配 --import-words）：写 `<词表主干>.qj`，与设置页导入同一套命名
    #[arg(long, requires = "import_words")]
    pub import_to: Option<PathBuf>,

    /// 回放 / 整句评测时打印前 N 条没命中首选的例子
    #[arg(long, default_value_t = 20)]
    pub misses: usize,

    /// 覆盖引擎里的调参常数，`名=值`，逗号分隔或多次给。名字：lambda / k / cap / discount（个人 n-gram 插值 λ / K / 封顶 / 三元折扣），
    /// transpose / substitute / extra / missing / typo-cap / correction / correction-transpose（敲错四类代价 / 个人折扣上限 / 整段纠错代价 / 转置单独一份），
    /// span / abbreviated-span / beam（整句搜索规模：每格候选数 / 简拼格候选数 / 束宽。规模参数不接受 0 与小数，浮点参数不接受 NaN 与 inf）
    #[arg(long, value_delimiter = ',')]
    pub tune: Vec<String>,

    /// 整句评测：读中文文本（一行一段，按标点切句、按词库转成全拼）或 `--eval-save` 冻结下来的三列文件，
    /// 冷启动喂给引擎看整句能不能还原原句；可给多个文件
    #[arg(long, num_args = 1..)]
    pub eval_text: Vec<PathBuf>,

    /// 把整句评测用到的句子集写成 `句子\t拼音\t上文` 三列文件，下次直接 `--eval-text` 它，保证比的是同一份句子
    #[arg(long)]
    pub eval_save: Option<PathBuf>,

    /// 实验专用：P2C 按完整拼音给整句路径打分；仅用于 --eval-text，不看上文
    #[arg(long, requires = "eval_text", conflicts_with_all = ["neural", "neural_async", "neural_context"])]
    pub eval_p2c: Option<PathBuf>,

    /// 实验专用：在同一份句子集上让 P2C 直接生成整句（不经词图、不经重排），量重排架构的上限代价
    #[arg(long, requires = "eval_text")]
    pub eval_generate: Option<PathBuf>,

    /// 将整句评测逐条结果写成 JSONL（含句子、候选和延迟）
    #[arg(long, requires = "eval_text")]
    pub eval_details: Option<PathBuf>,

    /// 冷启动字词评测：读取 JSONL，不加载个人配置或个人学习文件
    #[arg(long, requires = "cold_output", conflicts_with_all = ["user_dict", "config", "predict", "replay", "eval_text", "english_mode", "shuangpin", "code", "aux_table", "tune", "fuzzy", "inputs", "neural_async", "mine_log", "check_user_words", "import_words"])]
    pub eval_cold: Option<PathBuf>,

    /// 冷启动逐词结果 JSONL（拒绝覆盖）
    #[arg(long, requires = "eval_cold")]
    pub cold_output: Option<PathBuf>,

    /// 同时评测 P2C 生成及联合候选；不改变引擎原候选
    #[arg(long, requires = "eval_cold")]
    pub cold_model: Option<PathBuf>,

    /// 直接查询这些拼音后退出；不给则进入交互模式
    pub inputs: Vec<String>,
}
