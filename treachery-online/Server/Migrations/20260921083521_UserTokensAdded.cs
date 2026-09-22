using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Treachery.Server.Migrations
{
    /// <inheritdoc />
    public partial class UserTokensAdded : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "UserTokens",
                columns: table => new
                {
                    Token = table.Column<string>(type: "TEXT", maxLength: 64, nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    Created = table.Column<DateTimeOffset>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_UserTokens", x => x.Token);
                });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "UserTokens");
        }
    }
}
