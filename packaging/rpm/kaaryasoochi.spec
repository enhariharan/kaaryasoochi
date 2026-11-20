# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Fedora / COPR. Builds offline: Source1 is the vendor tarball (packaging/vendor.sh) attached to
# the GitHub release. See packaging/README.md.

# The release profile strips the binaries; there is no debuginfo to extract.
%global debug_package %{nil}
%global appid io.github.enhariharan.Kaaryasoochi

Name:           kaaryasoochi
Version:        0.1.0
Release:        1%{?dist}
Summary:        Schedule and manage cron-style jobs

# Kaaryasoochi is MIT; the rest covers the crates compiled in (packaging/debian/copyright).
License:        MIT AND Apache-2.0 AND Unicode-3.0 AND Zlib AND BSD-2-Clause AND BSD-3-Clause AND ISC AND BSL-1.0 AND CDLA-Permissive-2.0 AND MPL-2.0
URL:            https://github.com/enhariharan/kaaryasoochi
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz
Source1:        %{url}/releases/download/v%{version}/%{name}-%{version}-vendor.tar.xz

BuildRequires:  cargo >= 1.88
BuildRequires:  rust >= 1.88
BuildRequires:  binutils
BuildRequires:  gcc
BuildRequires:  lld
BuildRequires:  libxdo-devel
BuildRequires:  pkgconfig(gtk+-3.0)
BuildRequires:  pkgconfig(javascriptcoregtk-4.1)
BuildRequires:  pkgconfig(libsoup-3.0)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(webkit2gtk-4.1)
BuildRequires:  pkgconfig(zlib)
BuildRequires:  systemd-rpm-macros
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       hicolor-icon-theme
Recommends:     iproute
Recommends:     libnotify

%description
Kaaryasoochi schedules commands and scripts the way cron does, with a desktop
window (WebKitGTK) to create jobs, pick schedules and read each run's output:
one-shot, interval and crontab schedules, run history, retries, timeouts,
per-job time zones and catch-up of runs missed while the machine was off.

Jobs are run by a per-user systemd user service (kaaryasoochi-server), as that
user and never as root. The kaaryasoochi launcher enables it on first launch.

%prep
%autosetup -n %{name}-%{version}
tar -xJf %{SOURCE1}
# dx strips release binaries with the toolchain's rust-objcopy, which Fedora's rust does not ship.
# Build them unstripped; rpm strips the installed files.
sed -i 's/^strip = "symbols"$/strip = false/' Cargo.toml
grep -q '^strip = false$' Cargo.toml

%build
# Fedora's default RUSTFLAGS (codegen-units=1, full debuginfo, no strip) override the workspace's
# release profile and lld setting, which keep peak memory low; the debuginfo would be discarded.
unset RUSTFLAGS
packaging/build.sh

%install
packaging/install.sh %{buildroot} %{_prefix}
# Built unstripped (see %%prep) and without debuginfo, so rpm's debuginfo step does not strip them.
strip --strip-all %{buildroot}%{_bindir}/kaaryasoochi-admin \
  %{buildroot}%{_prefix}/lib/kaaryasoochi/app/kaaryasoochi-app \
  %{buildroot}%{_prefix}/lib/kaaryasoochi/web/server

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{name}.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/%{appid}.metainfo.xml

# Not enabled for every user (the default user preset disables it): each user's first launch does.
%post
%systemd_user_post kaaryasoochi-server.service

%preun
%systemd_user_preun kaaryasoochi-server.service

%files
%license LICENSE
%doc README.md
%{_bindir}/kaaryasoochi
%{_bindir}/kaaryasoochi-admin
%{_bindir}/kaaryasoochi-server
%{_prefix}/lib/kaaryasoochi/
%{_userunitdir}/kaaryasoochi-server.service
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/*/apps/%{name}.*
%{_metainfodir}/%{appid}.metainfo.xml

%changelog
* Fri Nov 20 2026 Hariharan Narayanan <enhariharan@gmail.com> - 0.1.0-1
- Initial package
