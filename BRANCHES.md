# Fork 分支约定

`origin` 是本仓库 `linmii/Winc`，`upstream` 是源仓库 `DEEIX-AI/Winc`。

| 本仓库分支 | 用途 | 更新来源 |
| --- | --- | --- |
| `dev` | 上游开发分支的镜像，不提交自定义修改 | `upstream/dev` |
| `main` | 上游稳定分支的镜像，不提交自定义修改 | `upstream/main` |
| `codex/dev` | 自定义功能与上游开发版的整合 | `dev` 和功能分支 |
| `codex/main` | 已验证的自定义整合版本，保留上游稳定分支历史 | `main` 和 `codex/dev` |

`dev` 和 `main` 同步后应与各自的上游分支指向相同提交。文档、打包配置和功能修改
都只进入自定义分支。

插件功能在 `codex/dev` 开发、验证后，随整个分支合入 `codex/main`。
已整合的旧插件功能分支不再保留。

## 同步上游

先确认工作区干净，再获取上游更新并快进镜像分支。若快进失败，先检查分叉原因，
不要直接强制推送或把自定义提交合进镜像分支。

```sh
rtk git fetch upstream
rtk git switch dev
rtk git merge --ff-only upstream/dev
rtk git switch main
rtk git merge --ff-only upstream/main
rtk git push origin dev main
```

然后按 `dev → codex/dev → codex/main` 整合。`codex/main` 同时合入 `main`，
保留上游稳定分支的发布历史。解决冲突并完成相关测试后推送：

```sh
rtk git switch codex/dev
rtk git merge dev
# 解决冲突、验证后：
rtk git push origin codex/dev

rtk git switch codex/main
rtk git merge main
rtk git merge codex/dev
# 解决冲突、验证后：
rtk git push origin codex/main
```

这些步骤需要手动执行；创建分支不会自动持续同步源仓库。

## 自定义功能整合

日常开发使用 `codex/dev`，也可以从它建立 `codex/<功能名>` 分支，完成后合回
`codex/dev`。验证完成后，将 `codex/dev` 整体合入 `codex/main`。

这条合并路径会把上游 `dev` 的更新一并带入 `codex/main`，因此它可能包含尚未
进入上游 `main` 的开发版代码。需要纯上游稳定版时，使用镜像分支 `main`。
功能分支的提交全部进入整合分支并推送后，可以删除本地和远程功能分支。

## Windows 便携版

推送 `codex/dev` 或 `codex/main` 会触发
**Windows portable (custom)**，生成带插件的便携版 Actions 构建产物。
`dev`、`main` 保留上游的原始工作流。打包和下载方法见 [release/README.md](release/README.md)。
