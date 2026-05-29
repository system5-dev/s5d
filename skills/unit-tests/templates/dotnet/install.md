  xUnit + coverlet (SoTA .NET unit testing). Coverlet ships with `dotnet test`.

  Add to test project (Directory.Build.props or per-csproj):
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.*" />
    <PackageReference Include="xunit" Version="2.*" />
    <PackageReference Include="xunit.runner.visualstudio" Version="2.*" />
    <PackageReference Include="coverlet.collector" Version="6.*" />
    <PackageReference Include="JunitXml.TestLogger" Version="*" />

  Threshold check:
    dotnet test --collect:"XPlat Code Coverage" \
                --settings coverlet.runsettings \
                --results-directory test-reports/dotnet \
                --logger "junit;LogFilePath=test-reports/dotnet/junit.xml"

  Run via skill:
    bash <skill>/scripts/run.sh --stacks dotnet
