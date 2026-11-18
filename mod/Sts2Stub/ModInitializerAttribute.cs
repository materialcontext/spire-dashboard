namespace MegaCrit.Sts2.Core.Modding;

/// <summary>
/// Stand-in for the game's attribute: names the static method the mod
/// loader calls once the mod's assembly is loaded.
/// </summary>
[System.AttributeUsage(System.AttributeTargets.Class)]
public sealed class ModInitializerAttribute(string methodName) : System.Attribute
{
    public string MethodName { get; } = methodName;
}
