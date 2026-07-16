# Ubuntu Rootfs 说明

此目录应包含 `ubuntu-rootfs.tar.gz` 文件。

## 构建方法

### 方法 1: 使用 Docker（推荐）

```bash
cd scripts
docker build -f Dockerfile.ubuntu-rootfs -t peng-rootfs .
docker run --rm -v ../app/src/main/assets:/out peng-rootfs
```

### 方法 2: 在 Linux 环境中直接构建

```bash
# 需要 debootstrap 和 qemu-user-static
sudo debootstrap --arch=arm64 --variant=minbase jammy ubuntu-rootfs http://ports.ubuntu.com/ubuntu-ports

# 安装软件包
sudo chroot ubuntu-rootfs apt-get update
sudo chroot ubuntu-rootfs apt-get install -y python3 nodejs ffmpeg sqlite3

# 打包
tar -czf ubuntu-rootfs.tar.gz ubuntu-rootfs
```

### 方法 3: 从 CDN 下载预构建版本

首次启动时，应用会自动从配置的 CDN 下载 rootfs。

## 必需软件包

- python3
- nodejs
- ffmpeg
- sqlite3
- git, curl, wget

## 预期大小

约 200-500MB（取决于安装的包）