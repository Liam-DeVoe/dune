using Microsoft.AspNetCore.Components;
using Treachery.Client.GenericComponents;

namespace Treachery.Client.OtherComponents.Log;

// A main phase's log, drawn from its report's visible messages. Messages a
// design doesn't draw itself go through Line, the report's plain line.
public abstract class PhaseLogBase : GameComponent
{
    [Parameter] public required List<Message> Messages { get; set; }

    [Parameter] public required RenderFragment<Message> Line { get; set; }
}
