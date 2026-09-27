#!/usr/bin/env bash
#
#
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "== 1/3 拉取上游 (kairusds/Hachimi-Edge) =="
git fetch upstream main

if git merge-base --is-ancestor upstream/main HEAD; then
    echo "上游没有新提交，已是最新。"
else
    echo "== 2/3 合并上游改动 =="
    git log --oneline HEAD..upstream/main | head -20
    if ! git merge --no-edit upstream/main; then
        cat <<'MSG'

合并出现冲突。处理办法：
  1. 编辑冲突文件（通常是我改过的这几个）：
       src/core/gui.rs
       src/core/hachimi.rs
       src/il2cpp/sql.rs
       src/il2cpp/hook/umamusume/mod.rs
       src/il2cpp/hook/umamusume/CharacterBuildInfo.rs
       assets/locales/{en,zh-cn,zh-tw}.yml
  2. git add <文件>
  3. git commit --no-edit

想放弃这次合并回到合并前：git merge --abort
MSG
        exit 1
    fi
    echo "合并完成。"
fi

if [ "${1:-}" = "--build" ]; then
    echo "== 3/3 构建（需要 Rust，约 4 分钟）=="
    if [ ! -d ../egui/crates/egui ] || [ ! -d ../egui-directx11 ]; then
        echo "缺少构建依赖：../egui 或 ../egui-directx11（上游 CI 用的补丁版 egui）。"
        echo "先按 docs/update-zh.md 把它们 clone 到 $ROOT/.. 下。"
        exit 1
    fi
    export PATH="$HOME/.cargo/bin:$PATH"
    export HACHIMI_IGNORE_DIRTY=true
    cp Cargo.toml Cargo.toml.syncbak && cp Cargo.lock Cargo.lock.syncbak
    trap 'mv -f Cargo.toml.syncbak Cargo.toml; mv -f Cargo.lock.syncbak Cargo.lock' EXIT
    cat >> Cargo.toml <<'EOF'
egui-directx11 = { path = '../egui-directx11' }
egui = { path = '../egui/crates/egui' }
egui_extras = { path = '../egui/crates/egui_extras' }
EOF
    cargo build --release --target-dir build
    echo
    echo "构建完成：$ROOT/build/release/hachimi.dll"
    echo "安装：备份游戏目录里的 cri_mana_vpx.dll，再用这个文件覆盖它。"
fi
