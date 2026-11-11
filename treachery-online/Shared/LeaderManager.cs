/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public static class LeaderManager
{
    public const int FirstId = 1000;
    public static List<Leader> Leaders { get; private set; }
    public static Messiah Messiah { get; } = new() { Id = FirstId, SkinId = 1098 };
    public static LeaderFetcher LeaderLookup { get; private set; } = new();
    public static HeroFetcher HeroLookup { get; private set; } = new();

    static LeaderManager()
    {
        Initialize();
    }

    private static void Initialize()
    {
        Leaders =
        [
            new Leader(1001) { Value = 5, Faction = Faction.Atreides, HeroType = HeroType.Normal },
            new Leader(1002) { Value = 5, Faction = Faction.Atreides, HeroType = HeroType.Normal },
            new Leader(1003) { Value = 4, Faction = Faction.Atreides, HeroType = HeroType.Normal },
            new Leader(1004) { Value = 2, Faction = Faction.Atreides, HeroType = HeroType.Normal },
            new Leader(1005) { Value = 1, Faction = Faction.Atreides, HeroType = HeroType.Normal },

            new Leader(1006) { Value = 5, Faction = Faction.BeneGesserit, HeroType = HeroType.Normal },
            new Leader(1007) { Value = 5, Faction = Faction.BeneGesserit, HeroType = HeroType.Normal },
            new Leader(1008) { Value = 5, Faction = Faction.BeneGesserit, HeroType = HeroType.Normal },
            new Leader(1009) { Value = 5, Faction = Faction.BeneGesserit, HeroType = HeroType.Normal },
            new Leader(1010) { Value = 5, Faction = Faction.BeneGesserit, HeroType = HeroType.Normal },

            new Leader(1011) { Value = 6, Faction = Faction.Emperor, HeroType = HeroType.Normal },
            new Leader(1012) { Value = 5, Faction = Faction.Emperor, HeroType = HeroType.Normal },
            new Leader(1013) { Value = 3, Faction = Faction.Emperor, HeroType = HeroType.Normal },
            new Leader(1014) { Value = 3, Faction = Faction.Emperor, HeroType = HeroType.Normal },
            new Leader(1015) { Value = 2, Faction = Faction.Emperor, HeroType = HeroType.Normal },

            new Leader(1016) { Value = 7, Faction = Faction.Fremen, HeroType = HeroType.Normal },
            new Leader(1017) { Value = 6, Faction = Faction.Fremen, HeroType = HeroType.Normal },
            new Leader(1018) { Value = 5, Faction = Faction.Fremen, HeroType = HeroType.Normal },
            new Leader(1019) { Value = 3, Faction = Faction.Fremen, HeroType = HeroType.Normal },
            new Leader(1020) { Value = 2, Faction = Faction.Fremen, HeroType = HeroType.Normal },

            new Leader(1021) { Value = 5, Faction = Faction.Guild, HeroType = HeroType.Normal },
            new Leader(1022) { Value = 3, Faction = Faction.Guild, HeroType = HeroType.Normal },
            new Leader(1023) { Value = 3, Faction = Faction.Guild, HeroType = HeroType.Normal },
            new Leader(1024) { Value = 2, Faction = Faction.Guild, HeroType = HeroType.Normal },
            new Leader(1025) { Value = 1, Faction = Faction.Guild, HeroType = HeroType.Normal },

            new Leader(1026) { Value = 6, Faction = Faction.Harkonnen, HeroType = HeroType.Normal },
            new Leader(1027) { Value = 4, Faction = Faction.Harkonnen, HeroType = HeroType.Normal },
            new Leader(1028) { Value = 3, Faction = Faction.Harkonnen, HeroType = HeroType.Normal },
            new Leader(1029) { Value = 2, Faction = Faction.Harkonnen, HeroType = HeroType.Normal },
            new Leader(1030) { Value = 1, Faction = Faction.Harkonnen, HeroType = HeroType.Normal },

            new Leader(1031) { Value = 4, Faction = Faction.Ixian, HeroType = HeroType.Normal },
            new Leader(1032) { Value = 5, Faction = Faction.Ixian, HeroType = HeroType.Normal },
            new Leader(1033) { Value = 5, Faction = Faction.Ixian, HeroType = HeroType.Normal },
            new Leader(1034) { Value = 2, Faction = Faction.Ixian, HeroType = HeroType.Normal },
            new Leader(1035) { Value = 1, Faction = Faction.Ixian, HeroType = HeroType.Normal },

            new Leader(1036) { Faction = Faction.Tleilaxu, HeroType = HeroType.VariableValue },
            new Leader(1037) { Value = 4, Faction = Faction.Tleilaxu, HeroType = HeroType.Normal },
            new Leader(1038) { Value = 3, Faction = Faction.Tleilaxu, HeroType = HeroType.Normal },
            new Leader(1039) { Value = 2, Faction = Faction.Tleilaxu, HeroType = HeroType.Normal },
            new Leader(1040) { Value = 1, Faction = Faction.Tleilaxu, HeroType = HeroType.Normal },

            new Leader(1041) { Value = 2, Faction = Faction.Choam, HeroType = HeroType.Normal },
            new Leader(1042) { Value = 3, Faction = Faction.Choam, HeroType = HeroType.Normal },
            new Leader(1043) { Value = 3, Faction = Faction.Choam, HeroType = HeroType.Normal },
            new Leader(1044) { Value = 4, Faction = Faction.Choam, HeroType = HeroType.Normal },
            new Leader(1045) { Value = 4, Faction = Faction.Choam, HeroType = HeroType.Normal },
            new Leader(1046) { Value = 2, Faction = Faction.Choam, HeroType = HeroType.Auditor },

            new Leader(1047) { Value = 2, Faction = Faction.Richese, HeroType = HeroType.Normal },
            new Leader(1048) { Value = 2, Faction = Faction.Richese, HeroType = HeroType.Normal },
            new Leader(1049) { Value = 3, Faction = Faction.Richese, HeroType = HeroType.Normal },
            new Leader(1050) { Value = 4, Faction = Faction.Richese, HeroType = HeroType.Normal },
            new Leader(1051) { Value = 5, Faction = Faction.Richese, HeroType = HeroType.Normal },

            new Leader(1052) { Value = 4, Faction = Faction.Ecaz, HeroType = HeroType.Normal },
            new Leader(1053) { Value = 3, Faction = Faction.Ecaz, HeroType = HeroType.Normal },
            new Leader(1054) { Value = 3, Faction = Faction.Ecaz, HeroType = HeroType.Normal },
            new Leader(1055) { Value = 2, Faction = Faction.Ecaz, HeroType = HeroType.Normal },
            new Leader(1056) { Value = 4, Faction = Faction.Ecaz, HeroType = HeroType.Normal },
            new Leader(1057) { Value = 6, Faction = Faction.Ecaz, HeroType = HeroType.Vidal },

            new Leader(1058) { Value = 5, Faction = Faction.Moritani, HeroType = HeroType.Normal },
            new Leader(1059) { Value = 4, Faction = Faction.Moritani, HeroType = HeroType.Normal },
            new Leader(1060) { Value = 2, Faction = Faction.Moritani, HeroType = HeroType.Normal },
            new Leader(1061) { Value = 4, Faction = Faction.Moritani, HeroType = HeroType.Normal },
            new Leader(1062) { Value = 1, Faction = Faction.Moritani, HeroType = HeroType.Normal }
        ];
    }

    public static IEnumerable<Leader> GetLeaders(Faction faction)
    {
        return Leaders.Where(l => l.Faction == faction);
    }

    public class LeaderFetcher : IFetcher<Leader>
    {
        public Leader? Find(int id)
        {
            if (id == -1)
                return null;
            return Leaders.SingleOrDefault(t => t.Id == id);
        }

        public int GetId(Leader? obj)
        {
            if (obj == null)
                return -1;
            return obj.Id;
        }
    }

    public class HeroFetcher : IFetcher<IHero>
    {
        public IHero? Find(int id)
        {
            if (id < 0)
                return null;
            if (id == Messiah.Id)
                return Messiah;
            if (id >= FirstId)
                return Leaders.SingleOrDefault(t => t.Id == id);
            return TreacheryCardManager.Lookup.Find(id);
        }

        public int GetId(IHero? obj)
        {
            if (obj == null)
                return -1;
            return obj.Id;
        }
    }
}