<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-log/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-log/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# VRC-LOG

VRChat Local Avatar ID Logger

## Important Notice

This project does **NOT** rip or steal avatars, and does **NOT** break the VRChat Terms of Service.  
It just scans your local logs and analytics files for avatar IDs and sends them to avatar search providers.  
If you don't want your avatars searchable you can request the providers below to blacklist them.

I **DO NOT** work with search providers that don't allow blacklisting, such as YAAS (part of the SAARs ripper project)

## Cache Encryption

VRChat recently added local avatar cache encryption, which broke the logger, and rippers :)  
I've updated the logger to use the Amplitude Analytics file, which only contains data when switching worlds.  
You should take a look at this file for the short time it contains data before uploading, it contains **A LOT
** of data...

**You can also add `--enable-sdk-log-levels` to your launch options to get more avatars more quickly.**

### VRCX Auto-Launch

This program prints [VRCX] avatar links when a new (to you) avatars get discovered.  
You can place a **shortcut** to this program within the [VRCX] Auto-Launch Folder (Settings > Advanced)

### Steam Launch Options (Headless)

Place the file in the VRChat directory or `PATH` and set your launch options  
`vrc-log(.exe) %command%`

### Supported Avatar Database Providers

- [avtrDB - Avatar Search] - [Discord](https://discord.gg/ZxB6w2hGfU) / [VRCX](https://api.avtrdb.com/v1/avatar/search/vrcx) / [Web](https://avtrdb.com)
- [VRCDB - Avatar Search] - [Discord](https://discord.gg/q427ecnUvj) / [VRCX](https://vrcx.vrcdb.com/avatars/Avatar/VRCX) / [Web](https://vrcdb.com) / [World](https://vrchat.com/home/world/wrld_1146f625-5d42-40f5-bfe7-06a7664e2796)
- [VRCDS - Project Dark Star] - [Discord](https://discord.gg/QT4uatfU8h) / [VRCX](https://avtr.nekosunevr.co.uk/vrcx_search.php) / [Web](https://avtr.nekosunevr.co.uk/search.php)
- [VRCWB - World Balancer] - [Discord](https://discord.gg/Uw7aAShdsp) / [VRCX](https://avatar.worldbalancer.com/vrcx_search.php) / [Web](https://avatar.worldbalancer.com/search.php)

#### Unsupported Avatar Database Providers

- ~~VRCDB - Ravenwood~~ - Shutdown
- ~~[Just H Party]~~ - Can't submit avatars
- ~~[Prismic's Avatar Search]~~ - Can't submit avatars

Additional contributions welcome, please open an issue, pull request, or join Discord.

[avtrDB - Avatar Search]: https://avtrdb.com

[Just H Party]: https://avtr.just-h.party

[Prismic's Avatar Search]: https://vrchat.com/home/world/wrld_57514404-7f4e-4aee-a50a-57f55d3084bf

[VRCDB - Avatar Search]: https://sites.smokes-hub.de

[VRCDS - Project Dark Star]: https://avtr.nekosunevr.co.uk

[VRCWB - World Balancer]: https://avatar.worldbalancer.com

[VRCX]: https://github.com/vrcx-team/VRCX?tab=readme-ov-file#--vrcx
