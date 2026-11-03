/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System.Net.Http;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.DependencyInjection;
using Treachery.Client.Scene;

namespace Treachery.Client;

public class Program
{
    public static async Task Main(string[] args)
    {
        var builder = WebAssemblyHostBuilder.CreateDefault(args);
        builder.RootComponents.Add<App>("#app");
        builder.Services.AddScoped(_ => new HttpClient { BaseAddress = new Uri(builder.HostEnvironment.BaseAddress) });

        // The /scene page (for screenshots) plays a saved game without a
        // server, so it gets a service that never connects. Everything else
        // gets the real client.
        builder.Services.AddSingleton<IGameService>(services =>
            IsScenePage(services.GetRequiredService<NavigationManager>())
                ? new SceneGameService()
                : ActivatorUtilities.CreateInstance<Client>(services));

        builder.Services.AddSingleton<Browser>();
        builder.Services.AddSingleton<MenuState>();
        await builder.Build().RunAsync();
    }

    private static bool IsScenePage(NavigationManager navigation)
    {
        var path = navigation.ToBaseRelativePath(navigation.Uri).Split('?', '#')[0].TrimEnd('/');
        return path.Equals("scene", StringComparison.OrdinalIgnoreCase);
    }
}
