//! 环境变量同步的纯逻辑单元测试喵~
//!
//! 这些测试完全不碰注册表与进程环境,只验证合并 / 展开 / 差量三件纯计算喵。

use std::collections::HashMap;

use meow_app_launcher::platform::env::{
    EnvEntry, EnvMap, EnvSync, expand, expand_all, merge, normalize,
};

/// 造一条不可展开的变量(REG_SZ)喵
fn sz(name: &str, value: &str) -> EnvEntry {
    EnvEntry {
        name: name.into(),
        value: value.into(),
        expand: false,
    }
}

/// 按名字取值,找不到就 panic 喵
fn get(entries: &[EnvEntry], name: &str) -> String {
    entries
        .iter()
        .find(|e| normalize(&e.name) == normalize(name))
        .unwrap_or_else(|| panic!("合并结果里没有 {name} 喵"))
        .value
        .clone()
}

#[test]
fn 变量名归一化到大写() {
    assert_eq!(normalize("Path"), "PATH");
    assert_eq!(normalize("temp"), "TEMP");
    assert_eq!(normalize("AlReAdY"), "ALREADY");
}

#[test]
fn 用户值顶掉同名系统值() {
    let system = vec![sz("EDITOR", "notepad"), sz("LANG", "en")];
    let user = vec![sz("editor", "vim")];
    let merged = merge(&system, &user);

    assert_eq!(get(&merged, "EDITOR"), "vim", "用户值应覆盖系统值");
    assert_eq!(get(&merged, "LANG"), "en");
    /* 同名只留一条,不然写进程环境时会互相打架喵 */
    assert_eq!(
        merged
            .iter()
            .filter(|e| normalize(&e.name) == "EDITOR")
            .count(),
        1
    );
}

#[test]
fn path拼接在系统之后() {
    let system = vec![EnvEntry::expanding(
        "Path",
        r"C:\Windows;C:\Windows\System32",
    )];
    let user = vec![EnvEntry::expanding("PATH", r"%USERPROFILE%\bin")];
    let merged = merge(&system, &user);

    assert_eq!(
        get(&merged, "PATH"),
        r"C:\Windows;C:\Windows\System32;%USERPROFILE%\bin",
        "用户 PATH 必须接在系统 PATH 后面,绝不能把系统段吃掉喵"
    );
}

#[test]
fn path拼接消化多余分号() {
    let system = vec![EnvEntry::expanding("PATH", r"C:\Windows;")];
    let user = vec![EnvEntry::expanding("PATH", r";C:\tools")];
    let merged = merge(&system, &user);
    assert_eq!(get(&merged, "PATH"), r"C:\Windows;C:\tools");
}

#[test]
fn path只有一侧时直接采用() {
    let only_user = merge(&[], &[EnvEntry::expanding("PATH", r"C:\tools")]);
    assert_eq!(get(&only_user, "PATH"), r"C:\tools");

    let only_system = merge(&[EnvEntry::expanding("PATH", r"C:\Windows")], &[]);
    assert_eq!(get(&only_system, "PATH"), r"C:\Windows");
}

#[test]
fn 系统键里的会话级临时目录被忽略() {
    let system = vec![
        sz("TEMP", r"%SystemRoot%\TEMP"),
        sz("TMP", r"%SystemRoot%\TEMP"),
        sz("PATH", r"C:\Windows"),
    ];
    let user = vec![];
    let merged = merge(&system, &user);

    assert!(
        merged.iter().all(|e| normalize(&e.name) != "TEMP"),
        "机器键的 TEMP 是给服务的喵"
    );
    assert!(merged.iter().all(|e| normalize(&e.name) != "TMP"));
    assert_eq!(get(&merged, "PATH"), r"C:\Windows");
}

#[test]
fn 用户在用户键给出临时目录时采用() {
    let system = vec![sz("TEMP", r"%SystemRoot%\TEMP")];
    let user = vec![sz("TEMP", r"C:\Users\me\AppData\Local\Temp")];
    let merged = merge(&system, &user);
    assert_eq!(get(&merged, "TEMP"), r"C:\Users\me\AppData\Local\Temp");
}

#[test]
fn 可展开标记取并集() {
    let system = vec![sz("PATH", r"C:\Windows")];
    let user = vec![EnvEntry::expanding("PATH", r"%USERPROFILE%\bin")];
    let merged = merge(&system, &user);
    let path = merged
        .iter()
        .find(|e| normalize(&e.name) == "PATH")
        .unwrap();
    assert!(path.expand, "任一侧需要展开,合并结果就得展开喵");
}

#[test]
fn 展开正常引用() {
    let lookup = |name: &str| (name == "USERPROFILE").then(|| r"C:\Users\me".to_string());
    assert_eq!(expand(r"%USERPROFILE%\bin", lookup), r"C:\Users\me\bin");
    assert_eq!(expand(r"%USERPROFILE%", lookup), r"C:\Users\me");
    assert_eq!(expand(r"a%USERPROFILE%b", lookup), r"aC:\Users\meb");
}

#[test]
fn 展开未定义引用保留字面量() {
    let none = |_: &str| None;
    assert_eq!(
        expand(r"%NOPE%\x", none),
        r"%NOPE%\x",
        "展开失败要留原文,不能吞掉喵"
    );
    assert_eq!(expand("%NOPE%", none), "%NOPE%");
}

#[test]
fn 展开半个百分号原样吐回() {
    let none = |_: &str| None;
    assert_eq!(expand("100%", none), "100%");
    assert_eq!(expand("50% 完成", none), "50% 完成");
    assert_eq!(expand("%%", none), "%%", "空变量名不该被吃掉喵");
    assert_eq!(expand("", none), "");
}

#[test]
fn 展开只认成对的百分号() {
    let lookup = |name: &str| (name == "A").then(|| "1".to_string());
    /* 前半个 % 找不到配对,直到后面的 %A% 才配对,顺序不能乱喵 */
    assert_eq!(expand("%A% %B%", lookup), "1 %B%");
}

#[test]
fn 展开只处理可展开类型() {
    let entries = vec![
        EnvEntry::expanding("EXPAND_ME", r"%REF%\bin"),
        sz("LITERAL", r"%REF%\bin"),
        sz("REF", r"C:\ref"),
    ];
    let out = expand_all(&entries, |_| None);

    assert_eq!(out["EXPAND_ME"], r"C:\ref\bin", "REG_EXPAND_SZ 要展开喵");
    assert_eq!(out["LITERAL"], r"%REF%\bin", "REG_SZ 里的百分号是字面量喵");
}

#[test]
fn 展开递归解析引用链() {
    let entries = vec![
        EnvEntry::expanding("A", r"%B%\x"),
        EnvEntry::expanding("B", r"%C%\y"),
        EnvEntry::expanding("C", r"C:\c"),
    ];
    let out = expand_all(&entries, |_| None);
    assert_eq!(out["A"], r"C:\c\y\x");
}

#[test]
fn 展开自我引用不死循环() {
    let entries = vec![EnvEntry::expanding("A", "%A%")];
    let out = expand_all(&entries, |_| None);
    assert_eq!(out["A"], "%A%", "递归到顶应退化成字面量,而不是转死喵");
}

#[test]
fn 展开注册表缺失时回落到进程环境() {
    let entries = vec![EnvEntry::expanding("COMBINED", r"%PROCESS_ONLY%\x")];
    let fallback = |name: &str| (name == "PROCESS_ONLY").then(|| "P".to_string());
    let out = expand_all(&entries, fallback);
    assert_eq!(out["COMBINED"], "P\\x");
}

#[test]
fn 展开结果的键已归一化() {
    let entries = vec![EnvEntry::expanding("MiXeD", "1")];
    let out: EnvMap = expand_all(&entries, |_| None);
    assert_eq!(out["MIXED"], "1");
}

#[test]
fn 首轮同步写入所有变量() {
    let mut sync = EnvSync::new();
    let fresh: EnvMap = HashMap::from([
        ("B".to_string(), "2".to_string()),
        ("A".to_string(), "1".to_string()),
    ]);

    let plan = sync.plan(&fresh);
    assert_eq!(
        plan.set,
        vec![("A".into(), "1".into()), ("B".into(), "2".into())]
    );
    assert!(plan.remove.is_empty(), "首轮没有历史可删喵");
    assert!(!plan.is_empty());
}

#[test]
fn 同步顺序稳定可复现() {
    let fresh: EnvMap = HashMap::from([
        ("Z".to_string(), "z".to_string()),
        ("A".to_string(), "a".to_string()),
    ]);
    let first = EnvSync::new().plan(&fresh);
    let second = EnvSync::new().plan(&fresh);
    assert_eq!(first.set, second.set, "排序后才能让日志与排障可复现喵");
}

#[test]
fn 变量消失进入删除列表() {
    let mut sync = EnvSync::new();
    sync.plan(&HashMap::from([
        ("KEEP".to_string(), "1".to_string()),
        ("DROP".to_string(), "2".to_string()),
    ]));

    let plan = sync.plan(&HashMap::from([("KEEP".to_string(), "9".to_string())]));
    assert_eq!(
        plan.remove,
        vec!["DROP".to_string()],
        "用户删掉的变量得从进程环境里抹掉喵"
    );
    assert_eq!(plan.set, vec![("KEEP".to_string(), "9".to_string())]);
}

#[test]
fn 重复同步同一份环境不产生删除() {
    let fresh: EnvMap = HashMap::from([("A".to_string(), "1".to_string())]);
    let mut sync = EnvSync::new();
    sync.plan(&fresh);
    let plan = sync.plan(&fresh);
    assert!(plan.remove.is_empty());
}
