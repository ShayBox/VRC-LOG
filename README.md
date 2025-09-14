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

### VRCX Auto-Launch

This program prints [VRCX] avatar links when a new (to you) avatars get discovered.  
You can place a **shortcut** to this program within the [VRCX] Auto-Launch Folder (Settings > Advanced)

### Steam Launch Options (Headless)

Place the file in the VRChat directory or `PATH` and set your launch options  
`vrc-log(.exe) %command% --enable-sdk-log-levels`

### Amplitude Analytics

VRChat now encrypts local avatar cache files, which temporarily broke both logging tools and rippers.  
The logger has been updated to use VRChat's Amplitude Analytics file instead, which contains avatar data when switching
between worlds.

**Important:** The Amplitude file contains extensive telemetry data beyond just avatar IDs.  
VRChat writes, uploads, writes again, uploads again, and clears it every time you switch worlds.
You should review this file's contents, as it includes significant amounts of personally identifiable data.

You can enable automatic clearing of the Amplitude file through the setup wizard,  
or by manually configuring the `clear_amplitude` option in the config file.

### Process Monitor (Windows)

It will install it if it's not installed using winget.  
If you launch the logger with admin it will launch Process Monitor pre-configured.  
You must manually close it to scan the collected avatars, it will re-open automatically again.

### Supported Avatar Database Providers

- [avtrDB - Avatar Search] - [Discord](https://discord.gg/ZxB6w2hGfU) / [VRCX](https://api.avtrdb.com/v1/avatar/search/vrcx) / [Web](https://avtrdb.com)
- [NSVR - NekoSune Community] - [VRCX](https://avtr.nekosunevr.co.uk/vrcx_search) / [Web](https://avtr.nekosunevr.co.uk)
- [PAW - Puppy's Avatar World] - [Discord](https://discord.gg/zHhs4nQYxX) / [VRCX](https://paw-api.amelia.fun/vrcx_search) / [Web](https://paw.amelia.fun)
- [VRCDB - Avatar Search] - [Discord](https://discord.gg/q427ecnUvj) / [VRCX](https://vrcx.vrcdb.com/avatars/Avatar/VRCX) / [Web](https://vrcdb.com) / [World](https://vrchat.com/home/world/wrld_1146f625-5d42-40f5-bfe7-06a7664e2796)
- [VRCWB - World Balancer] - [Discord](https://discord.gg/Uw7aAShdsp) / [VRCX](https://avatar.worldbalancer.com/vrcx_search.php) / [Web](https://avatar.worldbalancer.com/search.php)

#### Unsupported Avatar Database Providers

- ~~VRCDB - Ravenwood~~ - Shutdown
- ~~[Just H Party]~~ - Can't submit avatars
- ~~[Prismic's Avatar Search]~~ - Can't submit avatars

Additional contributions welcome, please open an issue, pull request, or join Discord.

[VRCX]: https://github.com/vrcx-team/VRCX?tab=readme-ov-file#--vrcx

[avtrDB - Avatar Search]: https://avtrdb.com

[NSVR - NekoSune Community]: https://avtr.nekosunevr.co.uk

[PAW - Puppy's Avatar World]: https://paw.amelia.fun

[VRCDB - Avatar Search]: https://vrcdb.com/

[VRCWB - World Balancer]: https://avatar.worldbalancer.com

[Just H Party]: https://avtr.just-h.party

[Prismic's Avatar Search]: https://vrchat.com/home/world/wrld_57514404-7f4e-4aee-a50a-57f55d3084bf
